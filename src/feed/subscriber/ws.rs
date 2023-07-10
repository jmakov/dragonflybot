pub mod binance;
pub mod bitstamp;

use std;

use async_trait;
use error_stack::{Result, ResultExt};

use crate::constants;
use crate::constants::feed;
use crate::error;
use crate::feed::client;


#[async_trait::async_trait]
pub trait Subscribe: Send {
    async fn subscribe_to_l2_snap(&mut self, instrument_name: &str) -> Result<(), error::SubscriberError>;
}

pub struct Subscriber<'a, T: feed::Feed> {
    pub client: client::ws::ClientManager<'a>,
    marker: std::marker::PhantomData<T>
}

impl<'a, T: feed::Feed> Subscriber<'a, T> {
    pub async fn new(feed_info: constants::FeedInfo<'a>) -> Result<Subscriber<'a, T>, error::SubscriberError> {
        let client = client::ws::ClientManager::new(feed_info).await
            .change_context(error::SubscriberError)?;
        Ok(Self{client, marker: std::marker::PhantomData})
    }
}