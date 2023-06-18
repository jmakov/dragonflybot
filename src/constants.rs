use strum;


pub const QUEUE_BUFFER_SIZE: usize = 1024 * 1024;

#[derive(strum::EnumCount, strum::EnumIter, Clone, Copy, Debug, strum::Display)]
pub enum Feed {
    BinanceSpot,
    BitstampSpot
}
impl Feed {
    pub fn get_details(&self) -> FeedInfo {
        match self {
            Self::BinanceSpot => FeedInfo{domain: domain::BINANCE, path: path::BINANCE, port: port::BINANCE, protocol: &Protocol::WEBSOCKETS},
            Self::BitstampSpot => FeedInfo{domain: domain::BITSTAMP, path: path::DEFAULT, port: port::DEFAULT, protocol: &Protocol::WEBSOCKETS},
        }
    }
    pub fn get_name_for_grpc_service(&self) -> String {
        match self {
            Self::BinanceSpot => "binance".to_owned(),
            Self::BitstampSpot => "bitstamp".to_owned()
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
pub mod queue_consumer {
    pub const TOP_N_BBO: usize = 10;
}
pub mod service {
    pub const GRPC_SERVER_PORT: usize = 50051;
}

mod domain {
    pub const BINANCE: &str = "stream.binance.com";
    pub const BITSTAMP: &str = "ws.bitstamp.net";
}
mod port {
    pub const DEFAULT: u16 = 443;
    pub const BINANCE: u16 = 9443;
}
mod path {
    pub const DEFAULT: &str = "";
    pub const BINANCE: &str = "/stream";
}