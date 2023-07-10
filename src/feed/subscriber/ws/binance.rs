use async_trait;
use error_stack::{Result, Report};
use serde_json;
use tracing;

use crate::constants::feed;
use crate::error;
use crate::feed::subscriber::ws;


#[async_trait::async_trait]
impl<'a> ws::Subscribe for ws::Subscriber<'a, feed::BinanceSpot> {
    async fn subscribe_to_l2_snap(&mut self, instrument_name: &str) -> Result<(), error::SubscriberError> {
        let rq = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": [
                format!("{}@depth20@100ms", instrument_name)
            ],
            "id": 1
        });
        self.client.send(&rq).await;

        // verify subscription succeeded
        match self.client.read_msg().await {
            Ok(msg) => {
                if msg != "{\"result\":null,\"id\":1}" {
                    return Err(Report::new(error::SubscriberError)
                        .attach_printable("Subscribing to channel failed")
                        .attach(msg))
                }
                tracing::info!("Subscribed to {}", instrument_name);
            }
            Err(e) => {
                return Err(Report::new(error::SubscriberError).attach(e))
            }
        }
        Ok(())
    }
}