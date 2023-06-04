use futures::{sink::SinkExt, Sink, Stream};
use idna;
use sasl::common::{ChannelBinding, Credentials};
use std::pin::Pin;
use std::str::FromStr;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
#[cfg(feature = "tls-native")]
use tokio_native_tls::TlsStream;
#[cfg(feature = "tls-rust")]
use tokio_rustls::client::TlsStream;
use tokio_stream::StreamExt;
use xmpp_parsers::{ns, Element, Jid};

use super::auth::auth;
use super::bind::bind;
use crate::happy_eyeballs::connect_with_srv;
use crate::starttls::starttls;
use crate::xmpp_codec::Packet;
use crate::xmpp_stream::{self, add_stanza_id};
use crate::{Error, ProtocolError};

/// A simple XMPP client connection
///
/// This implements the `futures` crate's [`Stream`](#impl-Stream) and
/// [`Sink`](#impl-Sink<Packet>) traits.
pub struct Client {
    stream: XMPPStream,
}

type XMPPStream = xmpp_stream::XMPPStream<TlsStream<TcpStream>>;

impl Client {
    /// Start a new XMPP client and wait for a usable session
    pub async fn new<P: Into<String>>(jid: &str, password: P) -> Result<Self, Error> {
        let jid = Jid::from_str(jid)?;
        let client = Self::new_with_jid(jid, password.into()).await?;
        Ok(client)
    }

    /// Start a new client given that the JID is already parsed.
    pub async fn new_with_jid(jid: Jid, password: String) -> Result<Self, Error> {
        let stream = Self::connect(jid.clone(), password.clone()).await?;
        Ok(Client { stream })
    }

    /// Get direct access to inner XMPP Stream
    pub fn into_inner(self) -> XMPPStream {
        self.stream
    }

    async fn connect(jid: Jid, password: String) -> Result<XMPPStream, Error> {
        let username = jid.clone().node().unwrap();
        let password = password;
        let domain = idna::domain_to_ascii(&jid.clone().domain()).map_err(|_| Error::Idna)?;

        // TCP connection
        let tcp_stream = connect_with_srv(&domain, "_xmpp-client._tcp", 5222).await?;

        // Unencryped XMPPStream
        let xmpp_stream =
            xmpp_stream::XMPPStream::start(tcp_stream, jid.clone(), ns::JABBER_CLIENT.to_owned())
                .await?;

        let xmpp_stream = if xmpp_stream.stream_features.can_starttls() {
            // TlsStream
            let tls_stream = starttls(xmpp_stream).await?;
            // Encrypted XMPPStream
            xmpp_stream::XMPPStream::start(tls_stream, jid.clone(), ns::JABBER_CLIENT.to_owned())
                .await?
        } else {
            return Err(Error::Protocol(ProtocolError::NoTls));
        };

        let creds = Credentials::default()
            .with_username(username)
            .with_password(password)
            .with_channel_binding(ChannelBinding::None);
        // Authenticated (unspecified) stream
        let stream = auth(xmpp_stream, creds).await?;
        // Authenticated XMPPStream
        let xmpp_stream =
            xmpp_stream::XMPPStream::start(stream, jid, ns::JABBER_CLIENT.to_owned()).await?;

        // XMPPStream bound to user session
        let xmpp_stream = bind(xmpp_stream).await?;
        Ok(xmpp_stream)
    }

    /// Get the client's bound JID (the one reported by the XMPP
    /// server).
    pub fn bound_jid(&self) -> &Jid {
        &self.stream.jid
    }

    /// Send stanza
    pub async fn send_stanza<E>(&mut self, stanza: E) -> Result<(), Error>
    where
        E: Into<Element>,
    {
        self.send(Packet::Stanza(add_stanza_id(
            stanza.into(),
            ns::JABBER_CLIENT,
        )))
        .await
    }

    /// End connection by sending `</stream:stream>`
    ///
    /// You may expect the server to respond with the same. This
    /// client will then drop its connection.
    pub async fn end(mut self) -> Result<(), Error> {
        self.send(Packet::StreamEnd).await?;

        // Wait for stream end from server
        while let Some(Ok(_)) = self.next().await {}

        Ok(())
    }
}

/// Incoming XMPP events
///
/// In an `async fn` you may want to use this with `use
/// futures::stream::StreamExt;`
impl Stream for Client {
    type Item = Result<Element, Error>;

    /// Low-level read on the XMPP stream
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.stream).poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Some(Ok(Packet::Stanza(stanza)))) => {
                    return Poll::Ready(Some(Ok(stanza)))
                }
                Poll::Ready(Some(Ok(Packet::Text(_)))) => {
                    // Ignore, retry
                }
                Poll::Ready(_) =>
                // Unexpected and errors, just end
                {
                    return Poll::Ready(None)
                }
            }
        }
    }
}

/// Outgoing XMPP packets
///
/// See `send_stanza()` for an `async fn`
impl Sink<Packet> for Client {
    type Error = Error;

    fn start_send(mut self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        Pin::new(&mut self.stream).start_send(item)
    }

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream).poll_ready(cx)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream).poll_close(cx)
    }
}
