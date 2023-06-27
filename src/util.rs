use rust_decimal;

use crate::constants;


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
impl Default for Order {
    fn default() -> Self {
        Self{
            feed: constants::Feed::BinanceSpot,
            price: rust_decimal::Decimal::from(0),
            amount: rust_decimal::Decimal::from(0)
        }
    }
}