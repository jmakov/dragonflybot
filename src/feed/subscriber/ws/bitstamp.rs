use async_trait::async_trait;
use error_stack::Result;
use serde_json;

use crate::error;
use crate::feed::client;
use crate::feed::subscriber;


pub struct FeedSubscriber {pub client: client::ws::ClientManager}

#[async_trait]
impl subscriber::ws::Subscriber for FeedSubscriber {
    async fn read_msg(&mut self) -> Result<String, error::ClientError> {self.client.read_msg().await}
    async fn subscribe_to_l2_snap(&mut self, instrument_name: &str) {
        let rq = serde_json::json!({
            "event": "bts:subscribe",
            "data": {
                "channel": format!("order_book_{}", instrument_name)
            }
        });
        self.client.send(&rq).await;
    }
}