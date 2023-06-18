//! Order book snap change forwarder
//!
//! This worker subscribes to order book snapshots and forwards the order book to
//! queue consumer only if anything in the order book has changed.

mod binance;
mod bitstamp;

use std::str::FromStr;

use async_trait;
use error_stack::Result;
use rust_decimal;

use crate::constants;
use crate::constants::queue_consumer;
use constants::listener::orderbook_snap_change_forwarder;
use crate::error;
use crate::feed::subscriber;
use crate::types;
use crate::util;


#[async_trait::async_trait]
pub trait ListenerManager: Send {
    fn get_listener(&self) -> &FeedListener;
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
        let msg_offset = self.get_listener().get_msg_offset_orderbook();
        if old_msg[msg_offset..] == new_msg[msg_offset..] {false} else {true}
    }
    fn parse_order(&self, feed: constants::Feed, order: Vec<gjson::Value>) -> Box<util::Order> {
        let price = rust_decimal::Decimal::from_str(order[0].str()).expect("Expected a string float");
        let amount = rust_decimal::Decimal::from_str(order[1].str()).expect("Expected a string float");

        //we might want to use a memory pool
        Box::new(util::Order{feed, price, amount})
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
    fn parse_orderbook_snap(&self, feed: constants::Feed, msg: &str) -> Box<util::OrderBook> {
        //we might want to get pre initialized arrays from the memory pool instead
        let mut asks: types::Orders = vec![];
        let mut bids: types::Orders = vec![];
        asks.reserve(queue_consumer::TOP_N_BBO);
        bids.reserve(queue_consumer::TOP_N_BBO);

        let bind_json_asks = gjson::get(msg, "data.asks");
        let bind_json_bids = gjson::get(msg, "data.bids");
        let json_asks = bind_json_asks.array();
        let json_bids = bind_json_bids.array();

        for ask in &json_asks[0..queue_consumer::TOP_N_BBO] {
            //Is this faster than iterating 2x over asks/bids (with a dedicated
            // price/amount json path) and accessing the price or amount field directly?
            asks.push(self.parse_order(feed, ask.array()));
        }
        for bid in &json_bids[0..queue_consumer::TOP_N_BBO] {
            bids.push(self.parse_order(feed, bid.array()));
        }

        //we might want to use a memory pool instead
        Box::new(util::OrderBook{asks, bids})
    }

    /// Read the message from the feed. The whole message is returned (concatenated frames).
    async fn read_msg(&mut self) -> Result<String, error::ClientError>;

    /// Entry point for the task, worker
    async fn run(&mut self) {
        let mut old_msg = orderbook_snap_change_forwarder::INIT_DUMMY_MSG.to_owned();
        let feed = self.get_listener().feed.to_owned();

        //skip first status msg so we can work only on subscribed msgs
        let _ = self.read_msg().await;

        loop {
            match self.read_msg().await {
                Ok(msg) => {
                    if self.has_orderbook_changed(&old_msg, &msg) {
                        let orderbook = self.parse_orderbook_snap(feed, &msg);
                        let feed_orderbook = util::FeedOrderBook{feed, orderbook};

                        let queue = &self.get_listener().queue;

                        //we might want to use a memory pool instead of `Box`ing `feed_orderbook`
                        queue.send(Box::new(feed_orderbook)).await.unwrap();

                        old_msg = msg;
                    }
                },
                Err(_) => {
                    break;
                }
            };
        }
    }
}
trait Getter: Send {
    fn get_feed(&self) -> &constants::Feed;
    fn get_msg_offset_orderbook(&self) -> usize;
    fn get_queue(&self) -> &types::QueueSender;
}
#[async_trait::async_trait]
trait MsgReader: Send {
    /// Read the message from the feed
    ///
    /// Returns only the whole message once all the frames are concatenated.
    async fn read_msg(&mut self) -> Result<String, error::ClientError>;
}

/// Builder for this worker
///
/// The builder returns a worker that can immediately start - everything that is needed is being
/// prepared and initialized here e.g. subscribing to feeds this worker needs.
pub struct Builder {
    feed: constants::Feed,
    instrument_name: String,
    queue: types::QueueSender
}
impl Builder {
    pub fn new(feed: constants::Feed, instrument_name: String, queue: types::QueueSender)
        -> Builder {Builder{feed, instrument_name, queue}}
    pub async fn subscribe(self) -> Box<dyn ListenerManager> {
        let mut subscriber = subscriber::ws::Builder::new(&self.feed)
            .connect()
            .await;
        subscriber.subscribe_to_l2_snap(&self.instrument_name).await;

        match self.feed {
            constants::Feed::BinanceSpot => Box::new(
                binance::ListenerManager {
                    listener: FeedListener {
                        feed: self.feed,
                        msg_offset_orderbook_start: orderbook_snap_change_forwarder::msg_offset_orderbook_start::BINANCE,
                        queue: self.queue},
                    subscriber
                }
            ),
            constants::Feed::BitstampSpot => Box::new(
                bitstamp::ListenerManager {
                    listener: FeedListener {
                        feed: self.feed,
                        msg_offset_orderbook_start: orderbook_snap_change_forwarder::msg_offset_orderbook_start::BITSTAMP,
                        queue: self.queue},
                    subscriber
                }
            )
        }
    }
}

///Hides fields into 1 struct so we avoid code duplication in `ListenerManager`s
pub struct FeedListener {
    pub feed: constants::Feed,
    pub msg_offset_orderbook_start: usize,
    pub queue: types::QueueSender

    // We could have add here also the subscriber for convenient access and reducing
    // code duplication. However it would require protected access in order to be
    // able to be shared amongst threads. Since this is on the hot path (calls the
    // `read_msg` method, we move it to the worker structure where we don't need
    // locking for accessing it.
    // pub subscriber: Box<dyn subscriber::ws::Subscriber>
}
impl Getter for FeedListener {
    fn get_feed(&self) -> &constants::Feed {&self.feed}
    fn get_msg_offset_orderbook(&self) -> usize {self.msg_offset_orderbook_start}
    fn get_queue(&self) -> &types::QueueSender {&self.queue}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ClientError;

    struct MockListenerManager{listener: FeedListener}

    #[async_trait::async_trait]
    impl ListenerManager for MockListenerManager {
        fn get_listener(&self) -> &FeedListener {&self.listener}
        async fn read_msg(&mut self) -> Result<String, ClientError> {Ok("".to_owned())}
    }

    fn get_mock_listener_manager() -> MockListenerManager {
        let (tx, _) = tokio::sync::mpsc::channel::<types::BoxedFeedOrderBook>(constants::QUEUE_BUFFER_SIZE);
        MockListenerManager{
            listener: FeedListener{
                feed: constants::Feed::BinanceSpot,
                msg_offset_orderbook_start: orderbook_snap_change_forwarder::msg_offset_orderbook_start::BINANCE,
                queue: tx
            }
        }
    }
    //test different cases for this method
    mod has_orderbook_changed {
        use super::*;

        #[test]
        fn test_orderbook_not_changed() {
            let new_msg = orderbook_snap_change_forwarder::INIT_DUMMY_MSG;
            let old_msg = orderbook_snap_change_forwarder::INIT_DUMMY_MSG;
            let listener_manager = get_mock_listener_manager();
            assert_eq!(listener_manager.has_orderbook_changed(new_msg, old_msg), false);
        }

        #[test]
        fn test_orderbook_has_changed() {
            let old_msg = orderbook_snap_change_forwarder::INIT_DUMMY_MSG;
            let new_msg = orderbook_snap_change_forwarder::INIT_DUMMY_MSG;
            new_msg.to_owned().push_str("new updated_field");
            let listener_manager = get_mock_listener_manager();
            assert_eq!(listener_manager.has_orderbook_changed(new_msg, old_msg), false);
        }
    }
}