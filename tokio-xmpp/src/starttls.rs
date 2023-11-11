use futures::{sink::SinkExt, stream::StreamExt};

#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
use {
    std::sync::Arc,
    tokio_rustls::{
        client::TlsStream,
        rustls::{ClientConfig, OwnedTrustAnchor, RootCertStore, ServerName},
        TlsConnector,
    },
    webpki_roots,
};

#[cfg(feature = "tls-native")]
use {
    native_tls::TlsConnector as NativeTlsConnector,
    tokio_native_tls::{TlsConnector, TlsStream},
};

use tokio::io::{AsyncRead, AsyncWrite};
use xmpp_parsers::{ns, Element};

use crate::xmpp_codec::Packet;
use crate::xmpp_stream::XMPPStream;
use crate::{Error, ProtocolError};

#[cfg(feature = "tls-native")]
async fn get_tls_stream<S: AsyncRead + AsyncWrite + Unpin>(
    xmpp_stream: XMPPStream<S>,
) -> Result<TlsStream<S>, Error> {
    let domain = xmpp_stream.jid.domain_str().to_owned();
    let stream = xmpp_stream.into_inner();
    let tls_stream = TlsConnector::from(NativeTlsConnector::builder().build().unwrap())
        .connect(&domain, stream)
        .await?;
    Ok(tls_stream)
}

#[cfg(all(feature = "tls-rust", not(feature = "tls-native")))]
async fn get_tls_stream<S: AsyncRead + AsyncWrite + Unpin>(
    xmpp_stream: XMPPStream<S>,
) -> Result<TlsStream<S>, Error> {
    let domain = xmpp_stream.jid.domain_str().to_owned();
    let domain = ServerName::try_from(domain.as_str())?;
    let stream = xmpp_stream.into_inner();
    let mut root_store = RootCertStore::empty();
    root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    }));
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let tls_stream = TlsConnector::from(Arc::new(config))
        .connect(domain, stream)
        .await?;
    Ok(tls_stream)
}

/// Performs `<starttls/>` on an XMPPStream and returns a binary
/// TlsStream.
pub async fn starttls<S: AsyncRead + AsyncWrite + Unpin>(
    mut xmpp_stream: XMPPStream<S>,
) -> Result<TlsStream<S>, Error> {
    let nonza = Element::builder("starttls", ns::TLS).build();
    let packet = Packet::Stanza(nonza);
    xmpp_stream.send(packet).await?;

    loop {
        match xmpp_stream.next().await {
            Some(Ok(Packet::Stanza(ref stanza))) if stanza.name() == "proceed" => break,
            Some(Ok(Packet::Text(_))) => {}
            Some(Err(e)) => return Err(e.into()),
            _ => {
                return Err(ProtocolError::NoTls.into());
            }
        }
    }

    get_tls_stream(xmpp_stream).await
}
