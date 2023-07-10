//! Aggregates top N BBO from multiple feeds and publishes to the gRPC service
//!
//! Consumes queue from `orderbook_snap_change_forwarder` listener.
use std;
use std::sync::Arc;

use rust_decimal;
use rust_decimal::prelude::ToPrimitive;
use strum::EnumCount;
use tokio::sync::{broadcast, mpsc};
use tracing;

use crate::constants;
use crate::constants::feed_aggregator;
use crate::service::grpc::server::orderbook;
use crate::types;
use crate::util;


pub struct Aggregator {
    pub queue_rx: mpsc::Receiver<types::BoxedFeedOrderBook>,
    pub queue_tx: Arc<broadcast::Sender<types::BoxedOrderbookSummary>>
}

impl Aggregator {
    /// Runs the aggregator task, should be run in it's own thread
    ///
    /// When there's backlog in the queue, we try to catch up to the latest market state before we
    /// run the calculations.
    pub fn run(&mut self) {
        const RESERVED_SIZE:usize = constants::Feed::COUNT * feed_aggregator::TOP_N_BBO;
        let mut orderbooks = get_initialized_orderbooks();
        let mut new_update_available = false;

        loop {
            // process backlog
            loop {
                match self.queue_rx.try_recv() {
                    Ok(feed_orderbook) => {
                        let feed_id = feed_orderbook.feed as usize;

                        // replace old order book reference with an updated one
                        orderbooks[feed_id] = feed_orderbook.orderbook;
                        new_update_available = true;
                    }
                    Err(mpsc::error::TryRecvError::Empty) => {
                        // when all the backlog is processed, we should have a snapshot of latest
                        // market state, continue with calculations
                        break
                    }
                    Err(e) => {tracing::error!("Cannot receive from queue: {}", e)}
                }
            }

            if new_update_available {
                // We could allocate the vector before the loop but since we're storing references,
                // we'd have lifetime problems (the borrow checker doesn't recognize `.clear()`
                // dropping the refs). Another approach is using `std::mem::transmute` but that's
                // in the domain of unsafe code. Perhaps use a small memory pool.
                let mut asks: Vec<&util::Order> = vec![];
                let mut bids: Vec<&util::Order> = vec![];
                let mut asks_grpc: Vec<orderbook::Level> = vec![];
                let mut bids_grpc: Vec<orderbook::Level> = vec![];
                asks.reserve(RESERVED_SIZE);
                bids.reserve(RESERVED_SIZE);
                asks_grpc.reserve(feed_aggregator::TOP_N_BBO);
                bids_grpc.reserve(feed_aggregator::TOP_N_BBO);

                // Get top of the book from all books.
                // We concatenate only top N asks/bids from all order books to get sorted top N.
                // For that to be true, asks/bids need to be ordered (which we observe in the data we
                // receive).
                for orderbook in orderbooks.iter() {
                    let sliced_asks = &orderbook.asks[0..feed_aggregator::TOP_N_BBO];
                    for order in sliced_asks.iter() { asks.push(order); }

                    let sliced_bids = &orderbook.bids[0..feed_aggregator::TOP_N_BBO];
                    for order in sliced_bids.iter() { bids.push(order); }
                }

                asks.sort_by_key(|order| order.price);
                bids.sort_by_key(|order| std::cmp::Reverse(order.price));

                for i in 0..feed_aggregator::TOP_N_BBO {
                    let ask = asks[i];
                    let bid = bids[i];

                    //consider using a memory pool
                    asks_grpc.push(
                        orderbook::Level {
                            exchange: ask.feed.feed_name_for_grpc_service().to_owned(),
                            price: ask.price.to_f64().unwrap(),
                            amount: ask.amount.to_f64().unwrap()
                        });
                    bids_grpc.push(
                        orderbook::Level {
                            exchange: bid.feed.feed_name_for_grpc_service().to_owned(),
                            price: bid.price.to_f64().unwrap(),
                            amount: bid.amount.to_f64().unwrap()
                        });
                }

                let spread = asks[0].price - bids[0].price;
                let orderbook_summary = orderbook::Summary {
                    spread: spread.to_f64().unwrap(),
                    asks: asks_grpc,
                    bids: bids_grpc
                };

                //The top_bbo aggregator could send a more general message suitable for multiple consumers.
                //If that would be needed, we could introduce a transformer for the stream e.g. each
                //stream consumer would have it's own (async) transformer (method).
                match self.queue_tx.send(Box::new(orderbook_summary)) {
                    Ok(_) => {
                        //msg is sent
                    }
                    Err(_) => {
                        //nobody subscribed to this broadcast yet
                    }
                }
                new_update_available = false;
            }
        }
    }
}

/// Initializes order books to highest asks and lowest bids
///
/// Since when the program starts, not all order books have the representable value of the market,
/// the ordered results would include default values i.e. price = 0 for e.g. asks. To prevent that,
/// we initialize the values to practically positive and negative infinities.
fn get_initialized_orderbooks() -> [util::OrderBookTopN; constants::Feed::COUNT]{
    let mut orderbooks = [util::OrderBookTopN::default(); constants::Feed::COUNT];

    for orderbook in &mut orderbooks {
        for order in &mut orderbook.asks {
            order.price = rust_decimal::Decimal::from(constants::ORDER_PRICE_INF);
        }
        for order in &mut orderbook.bids {
            order.price = rust_decimal::Decimal::from(-constants::ORDER_PRICE_INF);
        }
    }
    return orderbooks;
}