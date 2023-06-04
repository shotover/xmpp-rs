//! Components in XMPP are services/gateways that are logged into an
//! XMPP server under a JID consisting of just a domain name. They are
//! allowed to use any user and resource identifiers in their stanzas.
use futures::{sink::SinkExt, task::Poll, Sink, Stream};
use std::pin::Pin;
use std::str::FromStr;
use std::task::Context;
use tokio::net::TcpStream;
use xmpp_parsers::{ns, Element, Jid};

use super::happy_eyeballs::connect_to_host;
use super::xmpp_codec::Packet;
use super::xmpp_stream;
use super::Error;
use crate::xmpp_stream::add_stanza_id;

mod auth;

/// Component connection to an XMPP server
///
/// This simplifies the `XMPPStream` to a `Stream`/`Sink` of `Element`
/// (stanzas). Connection handling however is up to the user.
pub struct Component {
    /// The component's Jabber-Id
    pub jid: Jid,
    stream: XMPPStream,
}

type XMPPStream = xmpp_stream::XMPPStream<TcpStream>;

impl Component {
    /// Start a new XMPP component
    pub async fn new(jid: &str, password: &str, server: &str, port: u16) -> Result<Self, Error> {
        let jid = Jid::from_str(jid)?;
        let password = password.to_owned();
        let stream = Self::connect(jid.clone(), password, server, port).await?;
        Ok(Component { jid, stream })
    }

    async fn connect(
        jid: Jid,
        password: String,
        server: &str,
        port: u16,
    ) -> Result<XMPPStream, Error> {
        let password = password;
        let tcp_stream = connect_to_host(server, port).await?;
        let mut xmpp_stream =
            xmpp_stream::XMPPStream::start(tcp_stream, jid, ns::COMPONENT_ACCEPT.to_owned())
                .await?;
        auth::auth(&mut xmpp_stream, password).await?;
        Ok(xmpp_stream)
    }

    /// Send stanza
    pub async fn send_stanza(&mut self, stanza: Element) -> Result<(), Error> {
        self.send(add_stanza_id(stanza, ns::COMPONENT_ACCEPT)).await
    }

    /// End connection
    pub async fn send_end(&mut self) -> Result<(), Error> {
        self.close().await
    }
}

impl Stream for Component {
    type Item = Element;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Ready(Some(Ok(Packet::Stanza(stanza)))) => return Poll::Ready(Some(stanza)),
                Poll::Ready(Some(Ok(Packet::Text(_)))) => {
                    // retry
                }
                Poll::Ready(Some(Ok(_))) =>
                // unexpected
                {
                    return Poll::Ready(None)
                }
                Poll::Ready(Some(Err(_))) => return Poll::Ready(None),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl Sink<Element> for Component {
    type Error = Error;

    fn start_send(mut self: Pin<&mut Self>, item: Element) -> Result<(), Self::Error> {
        Pin::new(&mut self.stream)
            .start_send(Packet::Stanza(item))
            .map_err(|e| e.into())
    }

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream)
            .poll_ready(cx)
            .map_err(|e| e.into())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream)
            .poll_flush(cx)
            .map_err(|e| e.into())
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream)
            .poll_close(cx)
            .map_err(|e| e.into())
    }
}
