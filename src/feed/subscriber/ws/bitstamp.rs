use async_trait;
use error_stack::{IntoReport, Result, ResultExt, Report};
use serde_json;
use tracing;

use crate::constants::feed;
use crate::error;
use crate::feed::subscriber::ws;


#[async_trait::async_trait]
impl<'a> ws::Subscribe for ws::Subscriber<'a, feed::BitstampSpot> {
    async fn subscribe_to_l2_snap(&mut self, instrument_name: &str) -> Result<(), error::SubscriberError> {
        let rq = serde_json::json!({
            "event": "bts:subscribe",
            "data": {
                "channel": format!("order_book_{}", instrument_name)
            }
        });
        self.client.send(&rq).await;

        // verify subscription succeeded
        match self.client.read_msg().await {
            Ok(msg) => {
                let response: serde_json::Value = serde_json::from_str(&msg)
                    .into_report()
                    .change_context(error::SubscriberError)
                    .attach(msg)?;

                if response["event"] != "bts:subscription_succeeded" {
                    return Err(Report::new(error::SubscriberError)
                        .attach_printable("Subscribing to channel failed")
                        .attach(response))
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