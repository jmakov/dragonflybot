mod binance;
mod bitstamp;

use async_trait::async_trait;
use error_stack::Result;

use crate::constants;
use crate::error;
use crate::feed::client;



#[async_trait]
pub trait Subscriber: Send {
    async fn read_msg(&mut self) -> Result<String, error::ClientError>;
    async fn subscribe_to_l2_snap(&mut self, instrument_name: &str);
}

/// Builds a new, connected feed subscriber that's immediately usable
pub struct Builder<'a> {feed: &'a constants::Feed}

impl<'a> Builder<'a> {
    pub fn new(feed: &'a constants::Feed) -> Builder<'a> {Builder{feed}}
    pub async fn connect(&self) -> Box<dyn Subscriber> {
        let feed_info = self.feed.get_feed_info();
        let client = client::ws::ClientManager::new(&feed_info).await.unwrap();

        match self.feed {
            constants::Feed::BinanceSpot => Box::new(binance::FeedSubscriber{client}),
            constants::Feed::BitstampSpot => Box::new(bitstamp::FeedSubscriber{client}),
        }
    }
}