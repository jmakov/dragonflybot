use std::sync::Arc;

use rust_decimal;
use tokio::sync::broadcast;

use crate::constants;
use crate::types;


#[derive(Debug)]
pub struct FeedOrderBook {
    pub feed: constants::Feed,
    pub orderbook: OrderBookTopN
}
#[derive(Clone, Copy, Debug)]
pub struct Order {
    pub feed: constants::Feed,
    pub price: rust_decimal::Decimal,
    pub amount: rust_decimal::Decimal
}
impl Default for Order {
    fn default() -> Self {
        Self{
            feed: constants::Feed::BinanceSpot,
            price: rust_decimal::Decimal::from(0),
            amount: rust_decimal::Decimal::from(0)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OrderBookTopN {
    pub asks: [Order; constants::feed_aggregator::TOP_N_BBO],
    pub bids: [Order; constants::feed_aggregator::TOP_N_BBO]
}
impl Default for OrderBookTopN {
    fn default() -> Self {
        Self {
            asks: [Order::default(); constants::feed_aggregator::TOP_N_BBO],
            bids: [Order::default(); constants::feed_aggregator::TOP_N_BBO]
        }
    }
}
impl OrderBookTopN {
    pub fn set_unreachable_price(&mut self) {
        for order in &mut self.asks {
            order.price = rust_decimal::Decimal::from(constants::ORDER_PRICE_INF);
        }
        for order in &mut self.bids {
            order.price = rust_decimal::Decimal::from(-constants::ORDER_PRICE_INF);
        }
    }
}

pub struct GrpcClientContext {
    pub instrument_name: String,
    pub broadcast_aggregator_tx: Arc<broadcast::Sender<types::BoxedOrderbookSummary>>
}