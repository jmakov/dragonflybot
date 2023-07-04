use std;
use std::pin;

use tokio::sync::{broadcast, mpsc};
use tokio_stream;
use tokio_stream::wrappers;
use tonic;
use tracing;

use super::server::orderbook;
use super::server::orderbook::orderbook_aggregator_server;
use crate::constants;
use crate::util;


pub struct OrderbookAggregatorService {pub context: util::GrpcClientContext}

#[tonic::async_trait]
impl orderbook_aggregator_server::OrderbookAggregator for OrderbookAggregatorService {
    type BookSummaryStream =
        pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<orderbook::Summary, tonic::Status>> + Send + 'static>>;

    async fn book_summary(&self, _: tonic::Request<orderbook::Empty>)
                          -> Result<tonic::Response<Self::BookSummaryStream>, tonic::Status> {
        tracing::info!("New client connected");
        let mut broadcast_rx = self.context.broadcast_aggregator_tx.subscribe();
        let (queue_grpc_tx, queue_grpc_rx) =
            mpsc::channel::<Result::<orderbook::Summary, tonic::Status>>(constants::QUEUE_BUFFER_SIZE);

        tokio::spawn(
            async move {
                loop {
                    match broadcast_rx.recv().await {
                        Ok(orderbook_summary) => {
                            match queue_grpc_tx.send(Result::<_, tonic::Status>::Ok(*orderbook_summary)).await {
                                Ok(_) => {}
                                Err(_) => {
                                    //client disconnected
                                    tracing::info!("Client disconnected");
                                    break
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            //If we lag behind, keep retrying until we get to the most recent data.
                            loop {
                                match broadcast_rx.recv().await {
                                    Ok(orderbook_summary) => {
                                        break match queue_grpc_tx.send(Result::<_, tonic::Status>::Ok(*orderbook_summary)).await {
                                            Ok(_) => {}
                                            Err(_) => {
                                                //client disconnected
                                                tracing::info!("Client disconnected");
                                                break
                                            }
                                        }
                                    }
                                    Err(e) => { tracing::error!("Receiving from queue: {}", e); }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("Receiving from queue: {}", e);
                            match queue_grpc_tx.send(Result::<_, tonic::Status>::Err(
                                tonic::Status::new(tonic::Code::Internal, "Streaming error"))).await {
                                Ok(_) => {}
                                Err(_) => {
                                    //client disconnected
                                    tracing::info!("Client disconnected");
                                    break
                                }
                            }
                        }
                    }
                }
            }
        );
        let stream = wrappers::ReceiverStream::new(queue_grpc_rx);
        Ok(tonic::Response::new(Box::pin(stream) as Self::BookSummaryStream))
    }
}


