//! A gRPC server publishing top best bid&ask from aggregated order book
#![deny(unsafe_code)]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use clap::Parser;
use dragonflybot::{constants, error, service::grpc::orderbook_aggregator,
                          service::grpc::server::orderbook::orderbook_aggregator_server};
use error_stack::{IntoReport, Result, ResultExt};
use tonic;


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Instrument name to subscribe to
    #[arg(short, long)]
    instrument_name: String,
}

fn main() -> Result<(), error::Error> {
    let args = Args::parse();
    let addr = format!("0.0.0.0:{}", constants::service::GRPC_SERVER_PORT).parse().unwrap();

    let threaded_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .build()
        .into_report()
        .change_context(error::Error)
        .attach_printable("Cannot build threaded runtime")?;

    //start the root task from which we'll spawn new tasks
    threaded_runtime.block_on(
        tonic::transport::Server::builder()
            .add_service(
                orderbook_aggregator_server::OrderbookAggregatorServer::new(
                    orderbook_aggregator::OrderbookAggregatorService{
                        instrument_name: args.instrument_name.to_owned()}))
            .serve(addr)
    ).unwrap();

    Ok(())
}