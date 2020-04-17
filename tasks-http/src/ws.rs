use crate::reply::Reply;
use crate::{Error, Request, Response};
use futures::{future, ready, FutureExt, Sink, Stream, TryFuture, TryFutureExt};
use headers::{
    Connection, HeaderMapExt, SecWebsocketAccept, SecWebsocketKey, SecWebsocketVersion, Upgrade,
};
use http::Method;
use hyper::Body;
use modifier::Set;
use pin_project::{pin_project, project};
use std::borrow::Cow;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tasks::{reject, util, Rejection, Task};
use tokio_tungstenite::{
    tungstenite::protocol::{self, WebSocketConfig},
    WebSocketStream,
};

#[derive(Clone)]
pub struct WsTask<F> {
    inner: F,
    config: Option<WebSocketConfig>,
}

impl<F> WsTask<F> {
    /// Set the size of the internal message send queue.
    pub fn max_send_queue(mut self, max: usize) -> Self {
        self.config
            .get_or_insert_with(WebSocketConfig::default)
            .max_send_queue = Some(max);
        self
    }

    /// Set the maximum message size (defaults to 64 megabytes)
    pub fn max_message_size(mut self, max: usize) -> Self {
        self.config
            .get_or_insert_with(WebSocketConfig::default)
            .max_message_size = Some(max);
        self
    }

    /// Set the maximum frame size (defaults to 16 megabytes)
    pub fn max_frame_size(mut self, max: usize) -> Self {
        self.config
            .get_or_insert_with(|| WebSocketConfig::default())
            .max_frame_size = Some(max);
        self
    }
}

macro_rules! get_or_reject {
    ($req: expr, $pattern: expr) => {
        match $pattern {
            Some(out) => out,
            None => {
                return util::OneOf2Future::new(util::Promise::First(future::err(
                    Rejection::Reject($req),
                )))
            }
        }
    };
}

macro_rules! get_or_reject2 {
    ($req: expr, $pattern: expr) => {
        match $pattern {
            Some(out) => out,
            None => {
                panic!("out");
            }
        }
    };
}

pub fn ws<F>(cb: F) -> WsTask<F> {
    WsTask {
        inner: cb,
        config: None,
    }
}

impl<F, U, R, E> Task<Request> for WsTask<F>
where
    F: Clone + Send + Fn(Ws) -> U,
    U: Future<Output = Result<R, E>> + Send,
    R: Reply,
    E: std::error::Error + Send + Sync + 'static,
{
    type Output = R;
    type Error = Error;
    type Future = WsTaskFuture<F, U, R, E>;
    fn run(&self, req: Request) -> Self::Future {
        WsTaskFuture {
            state: WsTaskFutureState::Request(req, self.inner.clone()),
            config: self.config,
        }
    }
}

#[pin_project]
enum WsTaskFutureState<F, U, R, E>
where
    F: Fn(Ws) -> U,
    U: Future<Output = Result<R, E>>,
    R: Reply,
{
    Request(Request, F),
    Task(#[pin] U),
    Done,
}

#[pin_project]
pub struct WsTaskFuture<F, U, R, E>
where
    F: Fn(Ws) -> U,
    U: Future<Output = Result<R, E>>,
    R: Reply,
{
    #[pin]
    state: WsTaskFutureState<F, U, R, E>,
    config: Option<WebSocketConfig>,
}

impl<F, U, R, E> Future for WsTaskFuture<F, U, R, E>
where
    F: Fn(Ws) -> U,
    U: Future<Output = Result<R, E>>,
    R: Reply,
    E: std::error::Error + Send + Sync + 'static,
{
    type Output = Result<R, Rejection<Request, Error>>;
    #[project]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();

            #[project]
            match this.state.project() {
                WsTaskFutureState::Request(req, cb) => {
                    if req.method() != &Method::GET {
                        panic!("invalid");
                    }
                    let conn = get_or_reject2!(req, req.headers().typed_get::<Connection>());

                    if !conn.contains("upgrade") {}

                    let upgrade = get_or_reject2!(req, req.headers().typed_get::<Upgrade>());
                    if upgrade != Upgrade::websocket() {}

                    let version =
                        get_or_reject2!(req, req.headers().typed_get::<SecWebsocketVersion>());
                    if version != SecWebsocketVersion::V13 {}

                    let key = get_or_reject2!(req, req.headers().typed_get::<SecWebsocketKey>());

                    let body = req.take_body().unwrap_or(Body::empty());

                    let ws = Ws {
                        body: body,
                        config: this.config.clone(),
                        key: key,
                    };

                    let fut = cb(ws);

                    self.set(WsTaskFuture {
                        state: WsTaskFutureState::Task(fut),
                        config: self.config,
                    });
                }
                WsTaskFutureState::Task(fut) => match ready!(fut.poll(cx)) {
                    Ok(ret) => {
                        self.set(WsTaskFuture {
                            state: WsTaskFutureState::Done,
                            config: self.config,
                        });
                        return Poll::Ready(Ok(ret));
                    }
                    Err(err) => return Poll::Ready(Err(Rejection::Err(Error::new(err)))),
                },
                WsTaskFutureState::Done => panic!("poll after done"),
            }
        }
    }
}

pub struct Ws {
    body: ::hyper::Body,
    config: Option<WebSocketConfig>,
    key: SecWebsocketKey,
}

impl Ws {
    pub fn on_upgrade<F, U>(self, cb: F) -> impl Reply
    where
        F: FnOnce(WebSocket) -> U + Send + 'static,
        U: Future<Output = ()> + Send + 'static,
    {
        WsReply {
            ws: self,
            on_upgrade: cb,
        }
    }
}

// REPLY

#[allow(missing_debug_implementations)]
struct WsReply<F> {
    ws: Ws,
    on_upgrade: F,
}

impl<F, U> Reply for WsReply<F>
where
    F: FnOnce(WebSocket) -> U + Send + 'static,
    U: Future<Output = ()> + Send + 'static,
{
    fn into_response(self) -> Response {
        let on_upgrade = self.on_upgrade;
        let config = self.ws.config;
        let fut = self
            .ws
            .body
            .on_upgrade()
            .and_then(move |upgraded| {
                log::trace!("websocket upgrade complete");
                WebSocket::from_raw_socket(upgraded, protocol::Role::Server, config).map(Ok)
            })
            .and_then(move |socket| on_upgrade(socket).map(Ok))
            .map(|result| {
                if let Err(err) = result {
                    log::debug!("ws upgrade error: {}", err);
                }
            });
        ::tokio::task::spawn(fut);

        // let mut res = http::Response::default();

        // *res.status_mut() = http::StatusCode::SWITCHING_PROTOCOLS;

        // res.headers_mut().typed_insert(Connection::upgrade());
        // res.headers_mut().typed_insert(Upgrade::websocket());
        // res.headers_mut()
        //     .typed_insert(SecWebsocketAccept::from(self.ws.key));

        // res

        Response::with(http::StatusCode::SWITCHING_PROTOCOLS)
            .set(Connection::upgrade())
            .set(Upgrade::websocket())
            .set(SecWebsocketAccept::from(self.ws.key))
    }
}

/// A websocket `Stream` and `Sink`, provided to `ws` filters.
pub struct WebSocket {
    inner: WebSocketStream<hyper::upgrade::Upgraded>,
}

impl WebSocket {
    pub(crate) async fn from_raw_socket(
        upgraded: hyper::upgrade::Upgraded,
        role: protocol::Role,
        config: Option<protocol::WebSocketConfig>,
    ) -> Self {
        WebSocketStream::from_raw_socket(upgraded, role, config)
            .map(|inner| WebSocket { inner })
            .await
    }

    /// Gracefully close this websocket.
    pub async fn close(mut self) -> Result<(), crate::Error> {
        future::poll_fn(|cx| Pin::new(&mut self).poll_close(cx)).await
    }
}

impl Stream for WebSocket {
    type Item = Result<Message, crate::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match ready!(Pin::new(&mut self.inner).poll_next(cx)) {
            Some(Ok(item)) => Poll::Ready(Some(Ok(Message { inner: item }))),
            Some(Err(e)) => {
                log::debug!("websocket poll error: {}", e);
                Poll::Ready(Some(Err(crate::Error::new(e))))
            }
            None => {
                log::trace!("websocket closed");
                Poll::Ready(None)
            }
        }
    }
}

impl Sink<Message> for WebSocket {
    type Error = crate::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_ready(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(crate::Error::new(e))),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        match Pin::new(&mut self.inner).start_send(item.inner) {
            Ok(()) => Ok(()),
            Err(e) => {
                log::debug!("websocket start_send error: {}", e);
                Err(crate::Error::new(e))
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_flush(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(crate::Error::new(e))),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        match ready!(Pin::new(&mut self.inner).poll_close(cx)) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(err) => {
                log::debug!("websocket close error: {}", err);
                Poll::Ready(Err(crate::Error::new(err)))
            }
        }
    }
}

impl fmt::Debug for WebSocket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WebSocket").finish()
    }
}

/// A WebSocket message.
///
/// Only repesents Text and Binary messages.
///
/// This will likely become a `non-exhaustive` enum in the future, once that
/// language feature has stabilized.
#[derive(Eq, PartialEq, Clone)]
pub struct Message {
    inner: protocol::Message,
}

impl Message {
    /// Construct a new Text `Message`.
    pub fn text<S: Into<String>>(s: S) -> Message {
        Message {
            inner: protocol::Message::text(s),
        }
    }

    /// Construct a new Binary `Message`.
    pub fn binary<V: Into<Vec<u8>>>(v: V) -> Message {
        Message {
            inner: protocol::Message::binary(v),
        }
    }

    /// Construct a new Ping `Message`.
    pub fn ping<V: Into<Vec<u8>>>(v: V) -> Message {
        Message {
            inner: protocol::Message::Ping(v.into()),
        }
    }

    /// Construct the default Close `Message`.
    pub fn close() -> Message {
        Message {
            inner: protocol::Message::Close(None),
        }
    }

    /// Construct a Close `Message` with a code and reason.
    pub fn close_with(code: impl Into<u16>, reason: impl Into<Cow<'static, str>>) -> Message {
        Message {
            inner: protocol::Message::Close(Some(protocol::frame::CloseFrame {
                code: protocol::frame::coding::CloseCode::from(code.into()),
                reason: reason.into(),
            })),
        }
    }

    /// Returns true if this message is a Text message.
    pub fn is_text(&self) -> bool {
        self.inner.is_text()
    }

    /// Returns true if this message is a Binary message.
    pub fn is_binary(&self) -> bool {
        self.inner.is_binary()
    }

    /// Returns true if this message a is a Close message.
    pub fn is_close(&self) -> bool {
        self.inner.is_close()
    }

    /// Returns true if this message is a Ping message.
    pub fn is_ping(&self) -> bool {
        self.inner.is_ping()
    }

    /// Returns true if this message is a Pong message.
    pub fn is_pong(&self) -> bool {
        self.inner.is_pong()
    }

    /// Try to get a reference to the string text, if this is a Text message.
    pub fn to_str(&self) -> Result<&str, ()> {
        match self.inner {
            protocol::Message::Text(ref s) => Ok(s),
            _ => Err(()),
        }
    }

    /// Return the bytes of this message, if the message can contain data.
    pub fn as_bytes(&self) -> &[u8] {
        match self.inner {
            protocol::Message::Text(ref s) => s.as_bytes(),
            protocol::Message::Binary(ref v) => v,
            protocol::Message::Ping(ref v) => v,
            protocol::Message::Pong(ref v) => v,
            protocol::Message::Close(_) => &[],
        }
    }

    /// Destructure this message into binary data.
    pub fn into_bytes(self) -> Vec<u8> {
        self.inner.into_data()
    }
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl Into<Vec<u8>> for Message {
    fn into(self) -> Vec<u8> {
        self.into_bytes()
    }
}

// ===== Rejections =====

#[derive(Debug)]
pub(crate) struct MissingConnectionUpgrade;

impl ::std::fmt::Display for MissingConnectionUpgrade {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Connection header did not include 'upgrade'")
    }
}

impl ::std::error::Error for MissingConnectionUpgrade {}
