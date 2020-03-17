use futures::{sink::SinkExt, task::Poll, Future, Sink, Stream};
use idna;
use sasl::common::{ChannelBinding, Credentials};
use std::mem::replace;
use std::pin::Pin;
use std::str::FromStr;
use std::task::Context;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::task::LocalSet;
use tokio_tls::TlsStream;
use xmpp_parsers::{Element, Jid, JidParseError};

use super::event::Event;
use super::happy_eyeballs::connect;
use super::starttls::{starttls, NS_XMPP_TLS};
use super::xmpp_codec::Packet;
use super::xmpp_stream;
use super::{Error, ProtocolError};

mod auth;
mod bind;

/// XMPP client connection and state
///
/// This implements the `futures` crate's [`Stream`](#impl-Stream) and
/// [`Sink`](#impl-Sink<Packet>) traits.
pub struct Client {
    state: ClientState,
    jid: Jid,
    password: String,
    reconnect: bool,
}

type XMPPStream = xmpp_stream::XMPPStream<TlsStream<TcpStream>>;
const NS_JABBER_CLIENT: &str = "jabber:client";

enum ClientState {
    Invalid,
    Disconnected,
    Connecting(JoinHandle<Result<XMPPStream, Error>>, LocalSet),
    Connected(XMPPStream),
}

impl Client {
    /// Start a new XMPP client
    ///
    /// Start polling the returned instance so that it will connect
    /// and yield events.
    pub fn new<P: Into<String>>(jid: &str, password: P) -> Result<Self, JidParseError> {
        let jid = Jid::from_str(jid)?;
        let client = Self::new_with_jid(jid, password.into());
        Ok(client)
    }

    /// Start a new client given that the JID is already parsed.
    pub fn new_with_jid(jid: Jid, password: String) -> Self {
        let local = LocalSet::new();
        let connect = local.spawn_local(Self::connect(jid.clone(), password.clone()));
        let client = Client {
            jid,
            password,
            state: ClientState::Connecting(connect, local),
            reconnect: false,
        };
        client
    }

    /// Set whether to reconnect (`true`) or let the stream end
    /// (`false`) when a connection to the server has ended.
    pub fn set_reconnect(&mut self, reconnect: bool) -> &mut Self {
        self.reconnect = reconnect;
        self
    }

    async fn connect(jid: Jid, password: String) -> Result<XMPPStream, Error> {
        let username = jid.clone().node().unwrap();
        let password = password;
        let domain = idna::domain_to_ascii(&jid.clone().domain()).map_err(|_| Error::Idna)?;

        let tcp_stream = connect(&domain, Some("_xmpp-client._tcp"), 5222).await?;

        let xmpp_stream =
            xmpp_stream::XMPPStream::start(tcp_stream, jid, NS_JABBER_CLIENT.to_owned()).await?;
        let xmpp_stream = if Self::can_starttls(&xmpp_stream) {
            Self::starttls(xmpp_stream).await?
        } else {
            return Err(Error::Protocol(ProtocolError::NoTls));
        };

        let xmpp_stream = Self::auth(xmpp_stream, username, password).await?;
        let xmpp_stream = Self::bind(xmpp_stream).await?;
        Ok(xmpp_stream)
    }

    fn can_starttls<S: AsyncRead + AsyncWrite + Unpin>(
        xmpp_stream: &xmpp_stream::XMPPStream<S>,
    ) -> bool {
        xmpp_stream
            .stream_features
            .get_child("starttls", NS_XMPP_TLS)
            .is_some()
    }

    async fn starttls<S: AsyncRead + AsyncWrite + Unpin>(
        xmpp_stream: xmpp_stream::XMPPStream<S>,
    ) -> Result<xmpp_stream::XMPPStream<TlsStream<S>>, Error> {
        let jid = xmpp_stream.jid.clone();
        let tls_stream = starttls(xmpp_stream).await?;
        xmpp_stream::XMPPStream::start(tls_stream, jid, NS_JABBER_CLIENT.to_owned()).await
    }

    async fn auth<S: AsyncRead + AsyncWrite + Unpin + 'static>(
        xmpp_stream: xmpp_stream::XMPPStream<S>,
        username: String,
        password: String,
    ) -> Result<xmpp_stream::XMPPStream<S>, Error> {
        let jid = xmpp_stream.jid.clone();
        let creds = Credentials::default()
            .with_username(username)
            .with_password(password)
            .with_channel_binding(ChannelBinding::None);
        let stream = auth::auth(xmpp_stream, creds).await?;
        xmpp_stream::XMPPStream::start(stream, jid, NS_JABBER_CLIENT.to_owned()).await
    }

    async fn bind<S: Unpin + AsyncRead + AsyncWrite>(
        stream: xmpp_stream::XMPPStream<S>,
    ) -> Result<xmpp_stream::XMPPStream<S>, Error> {
        bind::bind(stream).await
    }

    /// Get the client's bound JID (the one reported by the XMPP
    /// server).
    pub fn bound_jid(&self) -> Option<&Jid> {
        match self.state {
            ClientState::Connected(ref stream) => Some(&stream.jid),
            _ => None,
        }
    }

    /// Send stanza
    pub async fn send_stanza(&mut self, stanza: Element) -> Result<(), Error> {
        self.send(Packet::Stanza(stanza)).await
    }

    /// End connection by sending `</stream:stream>`
    ///
    /// You may expect the server to respond with the same. This
    /// client will then drop its connection.
    ///
    /// Make sure to disable reconnect.
    pub async fn send_end(&mut self) -> Result<(), Error> {
        self.send(Packet::StreamEnd).await
    }
}

/// Incoming XMPP events
///
/// In an `async fn` you may want to use this with `use
/// futures::stream::StreamExt;`
impl Stream for Client {
    type Item = Event;

    /// Low-level read on the XMPP stream, allowing the underlying
    /// machinery to:
    ///
    /// * connect,
    /// * starttls,
    /// * authenticate,
    /// * bind a session, and finally
    /// * receive stanzas
    ///
    /// ...for your client
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let state = replace(&mut self.state, ClientState::Invalid);

        match state {
            ClientState::Invalid => panic!("Invalid client state"),
            ClientState::Disconnected if self.reconnect => {
                // TODO: add timeout
                let mut local = LocalSet::new();
                let connect =
                    local.spawn_local(Self::connect(self.jid.clone(), self.password.clone()));
                let _ = Pin::new(&mut local).poll(cx);
                self.state = ClientState::Connecting(connect, local);
                self.poll_next(cx)
            }
            ClientState::Disconnected => Poll::Ready(None),
            ClientState::Connecting(mut connect, mut local) => {
                match Pin::new(&mut connect).poll(cx) {
                    Poll::Ready(Ok(Ok(stream))) => {
                        let bound_jid = stream.jid.clone();
                        self.state = ClientState::Connected(stream);
                        Poll::Ready(Some(Event::Online {
                            bound_jid,
                            resumed: false,
                        }))
                    }
                    Poll::Ready(Ok(Err(e))) => {
                        self.state = ClientState::Disconnected;
                        return Poll::Ready(Some(Event::Disconnected(e.into())));
                    }
                    Poll::Ready(Err(e)) => {
                        self.state = ClientState::Disconnected;
                        panic!("connect task: {}", e);
                    }
                    Poll::Pending => {
                        let _ = Pin::new(&mut local).poll(cx);

                        self.state = ClientState::Connecting(connect, local);
                        Poll::Pending
                    }
                }
            }
            ClientState::Connected(mut stream) => {
                // Poll sink
                match Pin::new(&mut stream).poll_ready(cx) {
                    Poll::Pending => (),
                    Poll::Ready(Ok(())) => (),
                    Poll::Ready(Err(e)) => {
                        self.state = ClientState::Disconnected;
                        return Poll::Ready(Some(Event::Disconnected(e.into())));
                    }
                };

                // Poll stream
                match Pin::new(&mut stream).poll_next(cx) {
                    Poll::Ready(None) => {
                        // EOF
                        self.state = ClientState::Disconnected;
                        Poll::Ready(Some(Event::Disconnected(Error::Disconnected)))
                    }
                    Poll::Ready(Some(Ok(Packet::Stanza(stanza)))) => {
                        // Receive stanza
                        self.state = ClientState::Connected(stream);
                        Poll::Ready(Some(Event::Stanza(stanza)))
                    }
                    Poll::Ready(Some(Ok(Packet::Text(_)))) => {
                        // Ignore text between stanzas
                        self.state = ClientState::Connected(stream);
                        Poll::Pending
                    }
                    Poll::Ready(Some(Ok(Packet::StreamStart(_)))) => {
                        // <stream:stream>
                        self.state = ClientState::Disconnected;
                        Poll::Ready(Some(Event::Disconnected(
                            ProtocolError::InvalidStreamStart.into(),
                        )))
                    }
                    Poll::Ready(Some(Ok(Packet::StreamEnd))) => {
                        // End of stream: </stream:stream>
                        self.state = ClientState::Disconnected;
                        Poll::Ready(Some(Event::Disconnected(Error::Disconnected)))
                    }
                    Poll::Pending => {
                        // Try again later
                        self.state = ClientState::Connected(stream);
                        Poll::Pending
                    }
                    Poll::Ready(Some(Err(e))) => {
                        self.state = ClientState::Disconnected;
                        Poll::Ready(Some(Event::Disconnected(e.into())))
                    }
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
        match self.state {
            ClientState::Connected(ref mut stream) => {
                Pin::new(stream).start_send(item).map_err(|e| e.into())
            }
            _ => Err(Error::InvalidState),
        }
    }

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.state {
            ClientState::Connected(ref mut stream) => {
                Pin::new(stream).poll_ready(cx).map_err(|e| e.into())
            }
            _ => Poll::Pending,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.state {
            ClientState::Connected(ref mut stream) => {
                Pin::new(stream).poll_flush(cx).map_err(|e| e.into())
            }
            _ => Poll::Pending,
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match self.state {
            ClientState::Connected(ref mut stream) => {
                Pin::new(stream).poll_close(cx).map_err(|e| e.into())
            }
            _ => Poll::Pending,
        }
    }
}
