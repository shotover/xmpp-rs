use futures::{sink::SinkExt, task::Poll, Future, Sink, Stream};
use sasl::common::ChannelBinding;
use std::mem::replace;
use std::pin::Pin;
use std::task::Context;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use xmpp_parsers::{ns, Element, Jid};

use super::connect::{AsyncReadAndWrite, ServerConnector};
use crate::event::Event;
use crate::happy_eyeballs::{connect_to_host, connect_with_srv};
use crate::starttls::starttls;
use crate::xmpp_codec::Packet;
use crate::xmpp_stream::{self, add_stanza_id, XMPPStream};
use crate::{client_login, Error, ProtocolError};
#[cfg(feature = "tls-native")]
use tokio_native_tls::TlsStream;
#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
use tokio_rustls::client::TlsStream;

/// XMPP client connection and state
///
/// It is able to reconnect. TODO: implement session management.
///
/// This implements the `futures` crate's [`Stream`](#impl-Stream) and
/// [`Sink`](#impl-Sink<Packet>) traits.
pub struct Client<C: ServerConnector> {
    config: Config<C>,
    state: ClientState<C::Stream>,
    reconnect: bool,
    // TODO: tls_required=true
}

/// XMPP client configuration
#[derive(Clone, Debug)]
pub struct Config<C> {
    /// jid of the account
    pub jid: Jid,
    /// password of the account
    pub password: String,
    /// server configuration for the account
    pub server: C,
}

/// XMPP server connection configuration
#[derive(Clone, Debug)]
pub enum ServerConfig {
    /// Use SRV record to find server host
    UseSrv,
    #[allow(unused)]
    /// Manually define server host and port
    Manual {
        /// Server host name
        host: String,
        /// Server port
        port: u16,
    },
}

impl ServerConnector for ServerConfig {
    type Stream = TlsStream<TcpStream>;
    async fn connect(&self, jid: &Jid) -> Result<XMPPStream<Self::Stream>, Error> {
        // TCP connection
        let tcp_stream = match self {
            ServerConfig::UseSrv => {
                connect_with_srv(jid.domain_str(), "_xmpp-client._tcp", 5222).await?
            }
            ServerConfig::Manual { host, port } => connect_to_host(host.as_str(), *port).await?,
        };

        // Unencryped XMPPStream
        let xmpp_stream =
            xmpp_stream::XMPPStream::start(tcp_stream, jid.clone(), ns::JABBER_CLIENT.to_owned())
                .await?;

        if xmpp_stream.stream_features.can_starttls() {
            // TlsStream
            let tls_stream = starttls(xmpp_stream).await?;
            // Encrypted XMPPStream
            xmpp_stream::XMPPStream::start(tls_stream, jid.clone(), ns::JABBER_CLIENT.to_owned())
                .await
        } else {
            return Err(Error::Protocol(ProtocolError::NoTls));
        }
    }

    fn channel_binding(
        #[allow(unused_variables)] stream: &Self::Stream,
    ) -> Result<sasl::common::ChannelBinding, Error> {
        #[cfg(feature = "tls-native")]
        {
            log::warn!("tls-native doesn’t support channel binding, please use tls-rust if you want this feature!");
            Ok(ChannelBinding::None)
        }
        #[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
        {
            let (_, connection) = stream.get_ref();
            Ok(match connection.protocol_version() {
                // TODO: Add support for TLS 1.2 and earlier.
                Some(tokio_rustls::rustls::ProtocolVersion::TLSv1_3) => {
                    let data = vec![0u8; 32];
                    let data = connection.export_keying_material(
                        data,
                        b"EXPORTER-Channel-Binding",
                        None,
                    )?;
                    ChannelBinding::TlsExporter(data)
                }
                _ => ChannelBinding::None,
            })
        }
    }
}

enum ClientState<S: AsyncReadAndWrite> {
    Invalid,
    Disconnected,
    Connecting(JoinHandle<Result<XMPPStream<S>, Error>>),
    Connected(XMPPStream<S>),
}

impl Client<ServerConfig> {
    /// Start a new XMPP client
    ///
    /// Start polling the returned instance so that it will connect
    /// and yield events.
    pub fn new<J: Into<Jid>, P: Into<String>>(jid: J, password: P) -> Self {
        let config = Config {
            jid: jid.into(),
            password: password.into(),
            server: ServerConfig::UseSrv,
        };
        Self::new_with_config(config)
    }
}

impl<C: ServerConnector> Client<C> {
    /// Start a new client given that the JID is already parsed.
    pub fn new_with_config(config: Config<C>) -> Self {
        let connect = tokio::spawn(client_login(
            config.server.clone(),
            config.jid.clone(),
            config.password.clone(),
        ));
        let client = Client {
            config,
            state: ClientState::Connecting(connect),
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
        self.send(Packet::Stanza(add_stanza_id(stanza, ns::JABBER_CLIENT)))
            .await
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
impl<C: ServerConnector> Stream for Client<C> {
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
                let connect = tokio::spawn(client_login(
                    self.config.server.clone(),
                    self.config.jid.clone(),
                    self.config.password.clone(),
                ));
                self.state = ClientState::Connecting(connect);
                self.poll_next(cx)
            }
            ClientState::Disconnected => Poll::Ready(None),
            ClientState::Connecting(mut connect) => match Pin::new(&mut connect).poll(cx) {
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
                    self.state = ClientState::Connecting(connect);
                    Poll::Pending
                }
            },
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
                //
                // This needs to be a loop in order to ignore packets we don’t care about, or those
                // we want to handle elsewhere.  Returning something isn’t correct in those two
                // cases because it would signal to tokio that the XMPPStream is also done, while
                // there could be additional packets waiting for us.
                //
                // The proper solution is thus a loop which we exit once we have something to
                // return.
                loop {
                    match Pin::new(&mut stream).poll_next(cx) {
                        Poll::Ready(None) => {
                            // EOF
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(Error::Disconnected)));
                        }
                        Poll::Ready(Some(Ok(Packet::Stanza(stanza)))) => {
                            // Receive stanza
                            self.state = ClientState::Connected(stream);
                            return Poll::Ready(Some(Event::Stanza(stanza)));
                        }
                        Poll::Ready(Some(Ok(Packet::Text(_)))) => {
                            // Ignore text between stanzas
                        }
                        Poll::Ready(Some(Ok(Packet::StreamStart(_)))) => {
                            // <stream:stream>
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(
                                ProtocolError::InvalidStreamStart.into(),
                            )));
                        }
                        Poll::Ready(Some(Ok(Packet::StreamEnd))) => {
                            // End of stream: </stream:stream>
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(Error::Disconnected)));
                        }
                        Poll::Pending => {
                            // Try again later
                            self.state = ClientState::Connected(stream);
                            return Poll::Pending;
                        }
                        Poll::Ready(Some(Err(e))) => {
                            self.state = ClientState::Disconnected;
                            return Poll::Ready(Some(Event::Disconnected(e.into())));
                        }
                    }
                }
            }
        }
    }
}

/// Outgoing XMPP packets
///
/// See `send_stanza()` for an `async fn`
impl<C: ServerConnector> Sink<Packet> for Client<C> {
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
