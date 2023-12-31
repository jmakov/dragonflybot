//! A gRPC server publishing top best bid&ask from aggregated order book
#![deny(unsafe_code)]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::sync::Arc;

use clap::Parser;
use dragonflybot::{constants, error, feed, service::grpc::orderbook_aggregator,
                   service::grpc::server::orderbook::orderbook_aggregator_server, types, util};
use error_stack::{IntoReport, Result, ResultExt};
use tokio::sync::{broadcast, mpsc};
use tonic;
use tracing;
use tracing_subscriber;


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
    let instrument_name = args.instrument_name.to_owned();

    let logger = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(logger)
        .into_report()
        .change_context(error::Error)?;

    let threaded_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .build()
        .into_report()
        .change_context(error::Error)
        .attach_printable("Cannot build threaded runtime")?;

    let (queue_feed_listener_tx, queue_aggregator_rx) =
        mpsc::channel::<types::BoxedFeedOrderBook>(constants::QUEUE_BUFFER_SIZE);
    let (broadcast_tx, _) =
        broadcast::channel::<types::BoxedOrderbookSummary>(1);
    let broadcast_aggregator_tx = Arc::new(broadcast_tx);
    let broadcast_aggregator_tx_clone = Arc::clone(&broadcast_aggregator_tx);

    //spawn listeners
    let cloned_queue1 = queue_feed_listener_tx.clone();
    let cloned_queue2 = queue_feed_listener_tx.clone();
    let cloned_instrument_name1 = instrument_name.to_owned();
    let cloned_instrument_name2 = instrument_name.to_owned();

    threaded_runtime.spawn(
        async move {
            let mut listener = feed::listener::orderbook_snap_change_forwarder::Listener::<constants::feed::BinanceSpot>::new(
                constants::Feed::BinanceSpot,
                cloned_queue1,
                constants::listener::orderbook_snap_change_forwarder::msg_offset_orderbook_start::BINANCE,
                cloned_instrument_name1
            )
                .await.expect("Could not create new listener");
            let _ = listener.run().await;});
    threaded_runtime.spawn(
        async move {
            let mut listener = feed::listener::orderbook_snap_change_forwarder::Listener::<constants::feed::BitstampSpot>::new(
                constants::Feed::BitstampSpot,
                cloned_queue2,
                constants::listener::orderbook_snap_change_forwarder::msg_offset_orderbook_start::BITSTAMP,
                cloned_instrument_name2
            )
                .await.expect("Could not create new listener");
            let _ = listener.run().await;});


    //start the gRPC server
    threaded_runtime.spawn(
        tonic::transport::Server::builder()
            .add_service(
                orderbook_aggregator_server::OrderbookAggregatorServer::new(
                    orderbook_aggregator::OrderbookAggregatorService{
                        context: {util::GrpcClientContext {
                            instrument_name: instrument_name.to_owned(),
                            broadcast_aggregator_tx: broadcast_aggregator_tx_clone,
                        }
                    }}))
            .serve(addr)
    );

    // run the aggregator in it's own thread
    let handle_thread = std::thread::spawn(||{
        let mut listener_aggregator = feed::listener_aggregator::top_bbo::Aggregator {
            queue_rx: queue_aggregator_rx,
            queue_tx: broadcast_aggregator_tx
        };
        listener_aggregator.run();
    });
    handle_thread.join().unwrap();

    Ok(())
}