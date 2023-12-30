use sasl::common::{ChannelBinding, Credentials};
use tokio::io::{AsyncRead, AsyncWrite};
use xmpp_parsers::{ns, Jid};

use super::{auth::auth, bind::bind};
use crate::{xmpp_stream::XMPPStream, Error};

/// trait returned wrapped in XMPPStream by ServerConnector
pub trait AsyncReadAndWrite: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncReadAndWrite for T {}

/// Trait called to connect to an XMPP server, perhaps called multiple times
pub trait ServerConnector: Clone + core::fmt::Debug + Send + Unpin + 'static {
    /// The type of Stream this ServerConnector produces
    type Stream: AsyncReadAndWrite;
    /// This must return the connection ready to login, ie if starttls is involved, after TLS has been started, and then after the <stream headers are exchanged
    fn connect(
        &self,
        jid: &Jid,
    ) -> impl std::future::Future<Output = Result<XMPPStream<Self::Stream>, Error>> + Send;

    /// Return channel binding data if available
    /// do not fail if channel binding is simply unavailable, just return Ok(None)
    /// this should only be called after the TLS handshake is finished
    fn channel_binding(_stream: &Self::Stream) -> Result<ChannelBinding, Error> {
        Ok(ChannelBinding::None)
    }
}

/// Log into an XMPP server as a client with a jid+pass
/// does channel binding if supported
pub async fn client_login<C: ServerConnector>(
    server: C,
    jid: Jid,
    password: String,
) -> Result<XMPPStream<C::Stream>, Error> {
    let username = jid.node_str().unwrap();
    let password = password;

    let xmpp_stream = server.connect(&jid).await?;

    let channel_binding = C::channel_binding(xmpp_stream.stream.get_ref())?;

    let creds = Credentials::default()
        .with_username(username)
        .with_password(password)
        .with_channel_binding(channel_binding);
    // Authenticated (unspecified) stream
    let stream = auth(xmpp_stream, creds).await?;
    // Authenticated XMPPStream
    let xmpp_stream = XMPPStream::start(stream, jid, ns::JABBER_CLIENT.to_owned()).await?;

    // XMPPStream bound to user session
    let xmpp_stream = bind(xmpp_stream).await?;
    Ok(xmpp_stream)
}
