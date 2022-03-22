//! `XMPPStream` provides encoding/decoding for XMPP

use futures::sink::Send;
use futures::{sink::SinkExt, task::Poll, Sink, Stream};
use std::ops::DerefMut;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::Context;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;
use xmpp_parsers::{Element, Jid};

use crate::stream_features::StreamFeatures;
use crate::stream_start;
use crate::xmpp_codec::{Packet, XMPPCodec};
use crate::Error;

/// Wraps a binary stream (tokio's `AsyncRead + AsyncWrite`) to decode
/// and encode XMPP packets.
///
/// Implements `Sink + Stream`
pub struct XMPPStream<S: AsyncRead + AsyncWrite + Unpin> {
    /// The local Jabber-Id
    pub jid: Jid,
    /// Codec instance
    pub stream: Mutex<Framed<S, XMPPCodec>>,
    /// `<stream:features/>` for XMPP version 1.0
    pub stream_features: StreamFeatures,
    /// Root namespace
    ///
    /// This is different for either c2s, s2s, or component
    /// connections.
    pub ns: String,
    /// Stream `id` attribute
    pub id: String,
}

impl<S: AsyncRead + AsyncWrite + Unpin> XMPPStream<S> {
    /// Constructor
    pub fn new(
        jid: Jid,
        stream: Framed<S, XMPPCodec>,
        ns: String,
        id: String,
        stream_features: Element,
    ) -> Self {
        XMPPStream {
            jid,
            stream: Mutex::new(stream),
            stream_features: StreamFeatures::new(stream_features),
            ns,
            id,
        }
    }

    /// Send a `<stream:stream>` start tag
    pub async fn start(stream: S, jid: Jid, ns: String) -> Result<Self, Error> {
        let xmpp_stream = Framed::new(stream, XMPPCodec::new());
        stream_start::start(xmpp_stream, jid, ns).await
    }

    /// Unwraps the inner stream
    pub fn into_inner(self) -> S {
        self.stream.into_inner().unwrap().into_inner()
    }

    /// Re-run `start()`
    pub async fn restart(self) -> Result<Self, Error> {
        let stream = self.stream.into_inner().unwrap().into_inner();
        Self::start(stream, self.jid, self.ns).await
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin> XMPPStream<S> {
    /// Convenience method
    pub fn send_stanza<E: Into<Element>>(&mut self, e: E) -> Send<Self, Packet> {
        self.send(Packet::Stanza(e.into()))
    }
}

/// Proxy to self.stream
impl<S: AsyncRead + AsyncWrite + Unpin> Sink<Packet> for XMPPStream<S> {
    type Error = crate::Error;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Context) -> Poll<Result<(), Self::Error>> {
        // Pin::new(&mut self.stream).poll_ready(ctx)
        //     .map_err(|e| e.into())
        Poll::Ready(Ok(()))
    }

    fn start_send(
        #[cfg_attr(rustc_least_1_46, allow(unused_mut))] mut self: Pin<&mut Self>,
        item: Packet,
    ) -> Result<(), Self::Error> {
        Pin::new(&mut self.stream.lock().unwrap().deref_mut())
            .start_send(item)
            .map_err(|e| e.into())
    }

    fn poll_flush(
        #[cfg_attr(rustc_least_1_46, allow(unused_mut))] mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream.lock().unwrap().deref_mut())
            .poll_flush(cx)
            .map_err(|e| e.into())
    }

    fn poll_close(
        #[cfg_attr(rustc_least_1_46, allow(unused_mut))] mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.stream.lock().unwrap().deref_mut())
            .poll_close(cx)
            .map_err(|e| e.into())
    }
}

/// Proxy to self.stream
impl<S: AsyncRead + AsyncWrite + Unpin> Stream for XMPPStream<S> {
    type Item = Result<Packet, crate::Error>;

    fn poll_next(
        #[cfg_attr(rustc_least_1_46, allow(unused_mut))] mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream.lock().unwrap().deref_mut())
            .poll_next(cx)
            .map(|result| result.map(|result| result.map_err(|e| e.into())))
    }
}
