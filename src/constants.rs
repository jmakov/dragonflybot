use strum;


pub const QUEUE_BUFFER_SIZE: usize = 1024 * 1024;
pub const ORDER_PRICE_INF: i32 = 100_000_000; //a price not reachable by any fin. instrument

pub enum Protocol {
    // ARROWHEAD,  //Tokio Stock Exchange
    // BOE,    //Chicago options
    // FIX,
    // OUCH,
    // PILLAR, //NYSE
    // REST,
    // SAIL,   //Borsa Italia derivatives
    // T7ETI,  //Xetra (Frankfurt Stock Exchange)
    // UTP,    //Warshaw Stock Exchange
    WEBSOCKETS
}

pub mod listener {
    pub mod orderbook_snap_change_forwarder {
        pub const INIT_DUMMY_MSG: &str = "{\"data\":{\"timestamp\":\"1686616236\",\"microtimestamp\":\"1686616236740643\",\"bids\":[";

        pub mod msg_offset_orderbook_start {
            pub const BINANCE: usize = 76;
            pub const BITSTAMP: usize = 78;
        }
    }
}
pub mod feed_aggregator {
    pub const TOP_N_BBO: usize = 10;
}
pub mod service {
    pub const GRPC_SERVER_PORT: usize = 50051;
}

pub struct FeedInfo<'a> {
    pub domain: &'a str,
    pub path: &'a str,
    pub port: u16,
    pub protocol: Protocol
}

#[derive(strum::EnumCount, strum::EnumIter, Clone, Copy, Debug, strum::Display)]
pub enum Feed {
    BinanceSpot,
    BitstampSpot
}
impl Feed {
    pub fn feed_info(&self) -> FeedInfo<'static> {
        match self {
            Feed::BinanceSpot => FeedInfo{domain: "stream.binance.com", path: "/stream", port: 9443, protocol: Protocol::WEBSOCKETS},
            Feed::BitstampSpot => FeedInfo{domain: "ws.bitstamp.net", path: "", port: 443, protocol: Protocol::WEBSOCKETS},
        }
    }
    pub fn feed_name_for_grpc_service(&self) -> &str {
        match self {
            Feed::BinanceSpot => "binance",
            Feed::BitstampSpot => "bitstamp"
        }
    }
}

pub mod feed {
    // create a binding/contract so only these types can be used in listeners and subscribers
    pub trait Feed {}

    pub struct BinanceSpot {} impl Feed for BinanceSpot {}
    pub struct BitstampSpot {} impl Feed for BitstampSpot {}
}