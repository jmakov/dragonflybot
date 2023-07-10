use error_stack::{IntoReport, Result, ResultExt, Report};
use hyper::rt;

use crate::constants;
use crate::error;
use super::tls;


pub struct ClientManager<'a> {
    client: fastwebsockets::FragmentCollector<hyper::upgrade::Upgraded>,
    feed_info: constants::FeedInfo<'a>
}
struct SpawnExecutor;

impl<'a> ClientManager<'a> {
    pub async fn new(feed_info: constants::FeedInfo<'a>) -> Result<ClientManager<'a>, error::ClientError>  {
        Ok(ClientManager {
            client: get_ws_client(feed_info.domain, feed_info.port, feed_info.path).await?,
            feed_info})
    }

    /// Returns the whole message as received, as `String`
    ///
    /// In general we could work on single frames and thus avoid unneeded processing. However for some
    /// applications we just need the whole message. And that is what we return here - concatenated frames,
    /// from which we parse a string.
    pub async fn read_msg(&mut self) -> Result<String, error::ClientError> {
        match self.client.read_frame().await {
            Ok(frame) => {
                match frame.opcode {
                    fastwebsockets::OpCode::Text => {
                        String::from_utf8(frame.payload.to_vec())
                            .into_report()
                            .change_context(error::ClientError::ParsingError)
                            .attach_printable("Could not parse to string")
                            .attach(frame)
                    }
                    fastwebsockets::OpCode::Close => {
                        Err(Report::new(error::ClientError::EndpointClosedConnection))
                    }
                    _ => Err(
                        Report::new(error::ClientError::Error)
                            .attach_printable("Unexpected opcode")
                            .attach(frame.opcode)
                    )
                }
            }
            Err(e) => Err(
                Report::new(error::ClientError::Error)
                    .attach_printable("Cannot read frame")
                    .attach(e)
            )
        }
    }

    pub async fn reconnect(&mut self) -> Result<(), error::ClientError> {
        self.client = get_ws_client(self.feed_info.domain, self.feed_info.port, self.feed_info.path)
            .await?;
        Ok(())
    }

    pub async fn send(&mut self, msg: &serde_json::Value) {
        let _ = self.client.write_frame(
            fastwebsockets::Frame::text(msg.to_string().as_bytes().to_vec().into()))
            .await;
    }
}

// `tokio` executor needed only to process the initial handshake where we upgrade to WS protocol
impl<Fut> rt::Executor<Fut> for SpawnExecutor
    where
        Fut: std::future::Future + Send + 'static,
        Fut::Output: Send + 'static, {
    fn execute(&self, fut: Fut) {tokio::task::spawn(fut);}
}

/// Gets a connected web sockets client on a secure connection
///
/// In order to upgrade the connection to the WS protocol, we first get a secure stream
/// and upgrade the connection to the WS protocol with a handshake. As many servers
/// require a `pong` response on their `ping`, this is always enabled in this client.
async fn get_ws_client(domain: &str, port: u16, path: &str)
    -> Result<fastwebsockets::FragmentCollector<hyper::upgrade::Upgraded>, error::ClientError> {
    let addr = format!("{}:{}", domain, port);
    let tcp_stream = tokio::net::TcpStream::connect(&addr)
        .await
        .into_report()
        .change_context(error::ClientError::Error)
        .attach_printable("Establishing TCP stream failed")?;

    let domain_tls = tokio_rustls::rustls::ServerName::try_from(
        domain).map_err(|_| {
        Report::new(error::ClientError::Error).attach_printable("Invalid DNS name")})?;
    let tls_connector = tls::get_connector().unwrap();
    let tls_stream = tls_connector.connect(domain_tls, tcp_stream)
        .await
        .into_report()
        .change_context(error::ClientError::Error)
        .attach_printable("Could not establish TLS stream")?;
    let req = hyper::Request::builder()
        .method("GET")
        .uri(format!("wss://{}:{}{}", domain, port, path))
        .header("Host", &addr)
        .header(hyper::header::UPGRADE, "websocket")
        .header(hyper::header::CONNECTION, "upgrade")
        .header(
            "Sec-WebSocket-Key",
            fastwebsockets::handshake::generate_key(),
        )
        .header("Sec-WebSocket-Version", "13")
        .body(hyper::Body::empty())
        .into_report()
        .change_context(error::ClientError::Error)
        .attach_printable("Failed building request")?;
    let (mut ws, _) = fastwebsockets::handshake::client(
        &SpawnExecutor, req, tls_stream)
        .await
        .unwrap();
    ws.set_auto_pong(true);
    Ok(fastwebsockets::FragmentCollector::new(ws))
}