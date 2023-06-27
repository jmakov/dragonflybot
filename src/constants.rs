use strum;


pub const QUEUE_BUFFER_SIZE: usize = 1024 * 1024;
pub const ORDER_PRICE_INF: i32 = 100_000_000; //a price not reachable by any fin. instrument

#[derive(strum::EnumCount, strum::EnumIter, Clone, Copy, Debug, strum::Display)]
pub enum Feed {
    BinanceSpot,
    BitstampSpot
}
impl Feed {
    pub fn get_feed_info(&self) -> FeedInfo {
        match self {
            Self::BinanceSpot => FeedInfo{domain: "stream.binance.com", path: "/stream", port: 9443, protocol: &Protocol::WEBSOCKETS},
            Self::BitstampSpot => FeedInfo{domain: "ws.bitstamp.net", path: "", port: 443, protocol: &Protocol::WEBSOCKETS},
        }
    }
    pub fn get_feed_name_for_grpc_service(&self) -> &str {
        match self {
            Self::BinanceSpot => "binance",
            Self::BitstampSpot => "bitstamp"
        }
    }
}

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

pub struct FeedInfo<'a> {
    pub domain: &'a str,
    pub path: &'a str,
    pub port: u16,
    pub protocol: &'a Protocol
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
