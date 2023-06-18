use async_trait;
use error_stack::Result;

use crate::error;
use crate::feed::listener::orderbook_snap_change_forwarder;
use crate::feed::subscriber;


pub struct ListenerManager {
    pub listener: orderbook_snap_change_forwarder::FeedListener,
    pub subscriber: Box<dyn subscriber::ws::Subscriber>
}

#[async_trait::async_trait]
impl orderbook_snap_change_forwarder::ListenerManager for ListenerManager {
    fn get_listener(&self) -> &orderbook_snap_change_forwarder::FeedListener {&self.listener}
    async fn read_msg(&mut self) -> Result<String, error::ClientError> {self.subscriber.read_msg().await}
}