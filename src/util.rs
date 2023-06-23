use rust_decimal;

use crate::{constants, types};


#[derive(Debug)]
pub struct FeedOrderBook {
    pub feed: constants::Feed,
    pub orderbook: OrderBook
}
#[derive(Debug)]
pub struct Order {
    pub feed: constants::Feed,
    pub price: rust_decimal::Decimal,
    pub amount: rust_decimal::Decimal
}
#[derive(Debug)]
pub struct OrderBook {
    pub asks: types::Orders,
    pub bids: types::Orders
}

impl Default for OrderBook {
    fn default() -> Self {
        Self {
            asks: vec![Order::default()],
            bids: vec![Order::default()]
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