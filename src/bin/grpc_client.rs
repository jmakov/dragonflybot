//! Example gRPC client that prints received messages

use tokio;
use tokio_stream::StreamExt;
use tonic::transport;

use dragonflybot::{constants, error, service::grpc::server::orderbook};

type GrpcClient = orderbook::orderbook_aggregator_client::OrderbookAggregatorClient<transport::Channel>;


async fn print_stream(client: &mut GrpcClient) {
    let rq = orderbook::Empty{};
    let mut stream = client.book_summary(rq)
        .await
        .unwrap()
        .into_inner();

    while let Some(item) = stream.next().await {
        println!("{:?}", item.unwrap());
    }
}


#[tokio::main]
async fn main() -> Result<(), error::Error> {
    let mut client = GrpcClient::connect(
        format!("http://localhost:{}", constants::service::GRPC_SERVER_PORT)).await.unwrap();
    print_stream(&mut client).await;

    Ok(())
}