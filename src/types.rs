use tokio::sync::mpsc;
use tonic;

use crate::service::grpc::server::orderbook;
use crate::util;


pub type BoxedFeedOrderBook = Box<util::FeedOrderBook>;
pub type QueueGRPCSender = mpsc::Sender<BoxedOrderbookSummary>;
pub type QueueReceiver = mpsc::Receiver<BoxedFeedOrderBook>;
pub type QueueSender = mpsc::Sender<BoxedFeedOrderBook>;
pub type Orders = Vec<util::Order>;
pub type BoxedOrderbookSummary = Result<orderbook::Summary, tonic::Status>;