use crate::service::grpc::server::orderbook;
use crate::util;


pub type BoxedFeedOrderBook = Box<util::FeedOrderBook>;
pub type BoxedOrderbookSummary = Box<orderbook::Summary>;