//! Order book snap change forwarder
//!
//! This worker subscribes to order book snapshots and forwards the order book to
//! queue consumer only if anything in the order book has changed.

mod binance;
mod bitstamp;

use std;
use std::str::FromStr;

use async_trait;
use error_stack::{Result, ResultExt};
use rust_decimal;
use tokio;
use tokio::sync::mpsc;
use tracing;

use crate::constants;
use crate::constants::feed;
use crate::constants::feed_aggregator;
use constants::listener::orderbook_snap_change_forwarder;
use crate::error;
use crate::feed::subscriber::ws;
use crate::feed::subscriber::ws::Subscribe;
use crate::types;
use crate::util;


#[async_trait::async_trait]
pub trait ParseMsg: Send {
    fn parse_json_array_slice(&self, feed: constants::Feed, msg: &str, gjson_path: &str) -> [util::Order; feed_aggregator::TOP_N_BBO] {
        //we might want to use a memory pool instead
        let mut orders = [util::Order::default(); feed_aggregator::TOP_N_BBO];
        let value = gjson::get(msg, gjson_path);
        let mut i: usize = 0;

        value.each(|_, value| {
            let tpl = value.array();
            let order = &mut orders[i];
            order.amount = rust_decimal::Decimal::from_str(tpl[1].str()).expect("Expected a string float");
            order.feed = feed;
            order.price = rust_decimal::Decimal::from_str(tpl[0].str()).expect("Expected a string float");

            i += 1;
            if i == feed_aggregator::TOP_N_BBO {false} else {true}
        });
        return orders;
    }

    /// Parses a snap of the order book msg
    ///
    /// # Warning
    /// We assume some basic invariants on the data are always true:
    ///     - the order book snap msg always contains at least top N bids and asks
    ///     - bids and asks are always ordered in the msg
    ///     - bids and asks are ordered in the same way for all feeds we subscribe to
    ///
    /// We observe that to be the case from observing the data. However one could have a separate
    /// service that actually checks these invariants and if one gets violated, raises a signal/alarm.
    ///
    /// # Optimization considerations
    /// To get top N from the merged order book, we need to send only top N orders from each feed's
    /// order book.
    fn parse_orderbook_snap(&self, feed: constants::Feed, msg: &str) -> util::OrderBookTopN {
        let asks = self.parse_json_array_slice(feed, msg, "data.asks");
        let bids = self.parse_json_array_slice(feed, msg, "data.bids");
        util::OrderBookTopN {asks, bids}
    }
}


pub struct Listener<'a, T: feed::Feed> {
    feed: constants::Feed,
    instrument_name: String,
    msg_offset_orderbook_start: usize,
    subscriber: ws::Subscriber<'a, T>,
    queue_tx: mpsc::Sender<types::BoxedFeedOrderBook>
}

impl<'a, T: feed::Feed> Listener<'a, T>
where ws::Subscriber<'a, T>: Subscribe, Self: ParseMsg {
    pub async fn new(feed: constants::Feed, queue_tx: mpsc::Sender<types::BoxedFeedOrderBook>,
                     msg_offset_orderbook_start: usize, instrument_name: String)
        -> Result<Listener<'a, T>, error::ListenerError> {
        let subscriber = ws::Subscriber::<'a, T>::new(feed.feed_info()).await
                .change_context(error::ListenerError)?;
        Ok(Listener::<'a, T>{feed, msg_offset_orderbook_start, subscriber, queue_tx, instrument_name})
    }

    /// Notifies downstream to exclude this feed from the gRPC stream.
    ///
    /// In an event of an error or a reconnect, we don't want to propagate the error or stall the
    /// downstream updates causing calculations that result in states that don't represent the
    /// current market state. So in cases that require some time to recover e.g. reconnects, we
    /// update downstream with unreachable prices. Since downstream is sorting by (price, amount)
    /// this effectively causes our feed to be taken out of the resulting stream. And the gRPC
    /// client doesn't sees the stale data (no data for this feed until we recover).
    async fn exclude_listener_from_grpc_stream(&mut self) {
        tracing::warn!("Excluding feed from gRPC stream: {}", self.feed);

        let mut orderbook= util::OrderBookTopN::default();
        orderbook.set_unreachable_price();

        let feed_orderbook = util::FeedOrderBook{feed: self.feed.to_owned(), orderbook};

        match &self.queue_tx.send(Box::new(feed_orderbook)).await {
            Ok(_) => {}
            Err(e) => {tracing::error!("Cannot send item to queue: {}", e)}
        }
    }

    /// Detects if anything in the order book has changed
    ///
    ///  # Details
    /// We can reduce this problem to string slice comparison i.e. we don't need to waste resources
    /// parsing, ordering and comparing the data. Note that this holds only for certain feeds and might
    /// not be true in general. It can work for feeds where we notice that:
    ///     - bids and asks are already price ordered
    ///     - bid and ask fields are at predictable positions
    ///     - other fields (timestamp, instrument_name, etc.) are at predictable positions
    ///     - field positions aren't changing
    ///
    /// # Optimization opportunities
    /// Since we're interested only in the change in top of the order book, we could avoid comparing
    /// the whole message to the previous one but compare only the top of the message. If we decide
    /// to go that way, same invariants apply as mentioned in the #Details section.
    fn has_orderbook_changed(&self, old_msg: &str, new_msg: &str) -> bool {
        if old_msg[self.msg_offset_orderbook_start..] == new_msg[self.msg_offset_orderbook_start..]
            {false} else {true}
    }

    /// Entry point for the task - worker
    pub async fn run(&mut self) -> Result<(), error::ListenerError> {
        let mut old_msg = orderbook_snap_change_forwarder::INIT_DUMMY_MSG.to_owned();
        self.subscriber.subscribe_to_l2_snap(&self.instrument_name).await
            .change_context(error::ListenerError)?;

        loop {
            match self.subscriber.client.read_msg().await {
                Ok(msg) => {
                    if self.has_orderbook_changed(&old_msg, &msg) {
                        let orderbook = self.parse_orderbook_snap(self.feed.to_owned(), &msg);
                        let feed_orderbook = util::FeedOrderBook{feed: self.feed.to_owned(), orderbook};

                        //we might want to use a memory pool instead of `Box`ing `feed_orderbook`
                        match &self.queue_tx.send(Box::new(feed_orderbook)).await {
                            Ok(_) => {}
                            Err(e) => {tracing::error!("Cannot send item to queue: {}", e)}
                        }
                        old_msg = msg;
                    }
                },
                Err(e) => {
                    tracing::error!("Error reading from WebSockets: {}", e);
                    self.exclude_listener_from_grpc_stream().await;

                    match e.current_context() {
                        error::ClientError::EndpointClosedConnection => {
                            tracing::info!("WebSocket endpoint has closed the connection, attempting to reconnect to feed: {}", self.feed);

                            // try to reestablish previous state
                            self.subscriber.client.reconnect().await
                                .change_context(error::ListenerError)?;
                            self.subscriber.subscribe_to_l2_snap(&self.instrument_name).await
                                .change_context(error::ListenerError)?;
                        }
                        error::ClientError::Error => {tracing::error!("Could not read from websockets: {}", e)}
                        error::ClientError::ParsingError => {tracing::error!("Websockets msg could not be parsed: {}", e)}
                    }
                }
            };
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;


    //test different cases for this method
    mod has_orderbook_changed {
        use super::*;

        #[tokio::test]
        async fn test_orderbook_not_changed() {
            let new_msg = orderbook_snap_change_forwarder::INIT_DUMMY_MSG;
            let old_msg = orderbook_snap_change_forwarder::INIT_DUMMY_MSG;

            let (tx_binance, _) = mpsc::channel::<types::BoxedFeedOrderBook>(constants::QUEUE_BUFFER_SIZE);
            let tx_bitstamp = tx_binance.clone();
            let cloned_instrument_name1 = "btceth".to_owned();
            let cloned_instrument_name2 = cloned_instrument_name1.to_owned();
            let binance_spot = Listener::<feed::BinanceSpot>::new(
                constants::Feed::BinanceSpot,
                tx_binance,
                orderbook_snap_change_forwarder::msg_offset_orderbook_start::BINANCE,
                cloned_instrument_name1,
            ).await.expect("Cannot create listener");
            let bitstamp_spot = Listener::<feed::BitstampSpot>::new(
                constants::Feed::BitstampSpot,
                tx_bitstamp,
                orderbook_snap_change_forwarder::msg_offset_orderbook_start::BITSTAMP,
                cloned_instrument_name2
            ).await.expect("Cannot create listener");

            assert_eq!(binance_spot.has_orderbook_changed(&new_msg, &old_msg), false);
            assert_eq!(bitstamp_spot.has_orderbook_changed(&new_msg, &old_msg), false);
        }

        #[tokio::test]
        async fn test_orderbook_has_changed() {
            let (tx_binance, _) = mpsc::channel::<types::BoxedFeedOrderBook>(constants::QUEUE_BUFFER_SIZE);
            let tx_bitstamp = tx_binance.clone();
            let cloned_instrument_name1 = "btceth".to_owned();
            let cloned_instrument_name2 = cloned_instrument_name1.to_owned();
            let binance_spot = Listener::<feed::BinanceSpot>::new(
                constants::Feed::BinanceSpot,
                tx_binance,
                orderbook_snap_change_forwarder::msg_offset_orderbook_start::BINANCE,
                cloned_instrument_name1,
            ).await.expect("Cannot create listener");
            let bitstamp_spot = Listener::<feed::BitstampSpot>::new(
                constants::Feed::BitstampSpot,
                tx_bitstamp,
                orderbook_snap_change_forwarder::msg_offset_orderbook_start::BITSTAMP,
                cloned_instrument_name2
            ).await.expect("Cannot create listener");

            let old_msg = orderbook_snap_change_forwarder::INIT_DUMMY_MSG;
            let mut new_msg= old_msg.to_owned();
            new_msg.push_str("new updated_field");

            assert_eq!(binance_spot.has_orderbook_changed(&new_msg, &old_msg), true);
            assert_eq!(bitstamp_spot.has_orderbook_changed(&new_msg, &old_msg), true);
        }
    }
}