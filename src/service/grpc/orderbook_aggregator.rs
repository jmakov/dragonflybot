use std;
use std::pin;

use strum::IntoEnumIterator;
use tokio_stream;
use tokio_stream::wrappers;
use tonic;
use tokio::sync::mpsc;

use super::server::orderbook;
use super::server::orderbook::orderbook_aggregator_server;
use crate::listener_aggregator;
use crate::constants;
use crate::feed;
use crate::types;



#[derive(Debug)]
pub struct OrderbookAggregatorService {pub instrument_name: String }

#[tonic::async_trait]
impl orderbook_aggregator_server::OrderbookAggregator for OrderbookAggregatorService {
    type BookSummaryStream =
        pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<orderbook::Summary, tonic::Status>> + Send + 'static>>;

    async fn book_summary(&self, _: tonic::Request<orderbook::Empty>)
                          -> Result<tonic::Response<Self::BookSummaryStream>, tonic::Status> {
        let (tx_feed_listener, rx_orderbook_aggregator) =
            mpsc::channel::<types::BoxedFeedOrderBook>(constants::QUEUE_BUFFER_SIZE);
        let (tx_orderbook_aggregator, rx_grpc_service) =
            mpsc::channel::<types::BoxedOrderbookSummary>(constants::QUEUE_BUFFER_SIZE);

        //simply iterate feeds we're interested in for this service and spawn new feed listeners
        for feed in constants::Feed::iter() {
            let queue = tx_feed_listener.clone();
            let instrument_name = self.instrument_name.to_owned();

            tokio::spawn(
                async move {
                    let mut listener = feed::listener::orderbook_snap_change_forwarder::Builder::new(
                        feed, instrument_name, queue)
                        .subscribe()
                        .await;
                    listener.run().await;});}

        //spawn listener aggregator
        tokio::spawn(
            async move {
                let mut listener_aggregator = listener_aggregator::top_bbo::Aggregator {
                    queue_feed_listener_rx: rx_orderbook_aggregator,
                    queue_grpc_tx: tx_orderbook_aggregator
                };
                listener_aggregator.run().await;});

        let stream = wrappers::ReceiverStream::new(rx_grpc_service);
        Ok(tonic::Response::new(Box::pin(stream) as Self::BookSummaryStream))
    }
}