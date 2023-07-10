use async_trait;

use crate::constants::feed;
use crate::feed::listener::orderbook_snap_change_forwarder;


#[async_trait::async_trait]
impl<'a> orderbook_snap_change_forwarder::ParseMsg for orderbook_snap_change_forwarder::Listener<'a, feed::BitstampSpot> {}