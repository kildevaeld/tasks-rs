#[cfg(feature = "tls")]
use crate::tls::TlsConfigBuilder;
use std::convert::Infallible;
use std::error::Error as StdError;
use std::future::Future;
use std::net::SocketAddr;
#[cfg(feature = "tls")]
use std::path::Path;

use super::error::Error;
use futures::{future, FutureExt, TryFuture, TryStream, TryStreamExt};
use hyper::server::conn::AddrIncoming;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server as HyperServer;
use std::fmt;
use tokio::io::{AsyncRead, AsyncWrite};
#[cfg(all(unix, feature = "uds"))]
use tokio::net::UnixListener;
// use crate::filter::Filter;
// use crate::reject::IsReject;
// use crate::reply::Reply;
use super::{Request, Response};
// use super::TaskService;
use super::reply::Reply;
use crate::transport::Transport;
use tasks_core::Task;

/// Create a `Server` with the provided `Filter`.
pub fn serve<T>(task: T) -> Server<T>
where
    T: Task<Request, Error = Error> + Clone + Send + Sync + 'static,
    <T as Task<Request>>::Output: Reply,
    // F::Extract: Reply,
    // F::Error: IsReject
{
    Server {
        pipeline: false,
        task,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Protocol {
    Http,
    Https,
    Unix,
}

impl Protocol {
    pub fn as_str(&self) -> &str {
        match self {
            Protocol::Http => "http",
            Protocol::Https => "https",
            Protocol::Unix => "file",
        }
    }
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Http => write!(f, "http"),
            Protocol::Https => write!(f, "https"),
            Protocol::Unix => write!(f, "file"),
        }
    }
}

/// A Warp Server ready to filter requests.
#[derive(Debug)]
pub struct Server<F> {
    pipeline: bool,
    task: F,
}

/// A Warp Server ready to filter requests over TLS.
///
/// *This type requires the `"tls"` feature.*
#[cfg(feature = "tls")]
pub struct TlsServer<F> {
    server: Server<F>,
    tls: TlsConfigBuilder,
}

#[cfg(all(unix, feature = "uds"))]
pub struct UnixDomainServer<T> {
    server: Server<T>,
}

// Getting all various generic bounds to make this a re-usable method is
// very complicated, so instead this is just a macro.
macro_rules! into_service {
    ($into:expr, $protocol: expr, $local_addr: expr) => {{
        // let mut inner = crate::service($into);
        // inner.local_addr = $local_addr;
        // inner.protocol = $protocol;
        // make_service_fn(move |transport| {
        //     let inner = inner.clone();
        //     let remote_addr = Transport::remote_addr(transport);
        //     future::ok::<_, Infallible>(service_fn(move |req| {
        //         inner.call_with_addr(req, remote_addr)
        //     }))
        // })

        crate::MakeTaskService::new($protocol, $local_addr, $into)
    }};
    ($into:expr) => {{
        into_service!($into, Protocol::Http, None)
        // let inner = crate::service($into);
        // make_service_fn(move |transport| {
        //     let inner = inner.clone();
        //     let remote_addr = Transport::remote_addr(transport);
        //     future::ok::<_, Infallible>(service_fn(move |req| {
        //         inner.call_with_addr(req, remote_addr)
        //     }))
        // })
        // //let inner = crate::service($into);
        // let inner = $into;
        // make_service_fn(move |transport| {
        //     let inner = inner.clone();
        //     let remote_addr = Transport::remote_addr(transport);
        //     let service = TaskService::new($into);
        //     future::ok::<_, Infallible>(service)
        //     // future::ok::<_, Infallible>(service_fn(move |req| {
        //     //     inner.call_with_addr(req, remote_addr)
        //     // }))
        // })
    }};
}

macro_rules! addr_incoming {
    ($addr:expr) => {{
        let mut incoming = AddrIncoming::bind($addr)?;
        incoming.set_nodelay(true);
        let addr = incoming.local_addr();
        (addr, incoming)
    }};
}

macro_rules! bind_inner {
    ($this:ident, $addr:expr) => {{
        let (addr, incoming) = addr_incoming!($addr);
        let service = into_service!($this.task, Protocol::Http, Some(addr));
        let srv = HyperServer::builder(incoming)
            .http1_pipeline_flush($this.pipeline)
            .serve(service);
        Ok::<_, hyper::Error>((addr, srv))
    }};

    (tls: $this:ident, $addr:expr) => {{
        let (addr, incoming) = addr_incoming!($addr);
        let service = into_service!($this.server.task, Protocol::Https, Some(addr));
        let tls = $this.tls.build()?;
        let srv = HyperServer::builder(crate::tls::TlsAcceptor::new(tls, incoming))
            .http1_pipeline_flush($this.server.pipeline)
            .serve(service);
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((addr, srv))
    }};
}

macro_rules! bind {
    ($this:ident, $addr:expr) => {{
        let addr = $addr.into();
        (|addr| bind_inner!($this, addr))(&addr).unwrap_or_else(|e| {
            panic!("error binding to {}: {}", addr, e);
        })
    }};

    (tls: $this:ident, $addr:expr) => {{
        let addr = $addr.into();
        (|addr| bind_inner!(tls: $this, addr))(&addr).unwrap_or_else(|e| {
            panic!("error binding to {}: {}", addr, e);
        })
    }};
}

macro_rules! try_bind {
    ($this:ident, $addr:expr) => {{
        (|addr| bind_inner!($this, addr))($addr)
    }};

    (tls: $this:ident, $addr:expr) => {{
        (|addr| bind_inner!(tls: $this, addr))($addr)
    }};
}

// ===== impl Server =====

impl<T> Server<T>
where
    T: Task<Request, Error = Error> + Clone + Send + Sync + 'static,
    <T as Task<Request>>::Output: Reply,
    // <F::Future as TryFuture>::Ok: Reply,
    // <F::Future as TryFuture>::Error: IsReject,
{
    /// Run this `Server` forever on the current thread.
    pub async fn run(self, addr: impl Into<SocketAddr> + 'static) {
        let (addr, fut) = self.bind_ephemeral(addr);

        log::info!("listening on http://{}", addr);

        fut.await;
    }

    /// Run this `Server` forever on the current thread with a specific stream
    /// of incoming connections.
    ///
    /// This can be used for Unix Domain Sockets, or TLS, etc.
    pub async fn run_incoming<I>(self, incoming: I)
    where
        I: TryStream + Send,
        I::Ok: AsyncRead + AsyncWrite + Send + 'static + Unpin,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        self.run_incoming2(incoming.map_ok(crate::transport::LiftIo).into_stream())
            .await;
    }

    async fn run_incoming2<I>(self, incoming: I)
    where
        I: TryStream + Send,
        I::Ok: Transport + Send + 'static + Unpin,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let fut = self.serve_incoming2(incoming);

        log::info!("listening with custom incoming");

        fut.await;
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on any runtime.
    ///
    /// # Panics
    ///
    /// Panics if we are unable to bind to the provided address.
    pub fn bind(self, addr: impl Into<SocketAddr> + 'static) -> impl Future<Output = ()> + 'static {
        let (_, fut) = self.bind_ephemeral(addr);
        fut
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on any runtime.
    ///
    /// In case we are unable to bind to the specified address, resolves to an
    /// error and logs the reason.
    pub async fn try_bind(self, addr: impl Into<SocketAddr> + 'static) {
        let addr = addr.into();
        let srv = match try_bind!(self, &addr) {
            Ok((_, srv)) => srv,
            Err(err) => {
                log::error!("error binding to {}: {}", addr, err);
                return;
            }
        };

        srv.map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        })
        .await;
    }

    /// Bind to a possibly ephemeral socket address.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    ///
    /// # Panics
    ///
    /// Panics if we are unable to bind to the provided address.
    pub fn bind_ephemeral(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) -> (SocketAddr, impl Future<Output = ()> + 'static) {
        let (addr, srv) = bind!(self, addr);
        let srv = srv.map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        });

        (addr, srv)
    }

    /// Tried to bind a possibly ephemeral socket address.
    ///
    /// Returns a `Result` which fails in case we are unable to bind with the
    /// underlying error.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    pub fn try_bind_ephemeral(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) -> Result<(SocketAddr, impl Future<Output = ()> + 'static), crate::Error> {
        let addr = addr.into();
        let (addr, srv) = try_bind!(self, &addr).map_err(crate::Error::new)?;
        let srv = srv.map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        });

        Ok((addr, srv))
    }

    /// Create a server with graceful shutdown signal.
    ///
    /// When the signal completes, the server will start the graceful shutdown
    /// process.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    ///
    pub fn bind_with_graceful_shutdown(
        self,
        addr: impl Into<SocketAddr> + 'static,
        signal: impl Future<Output = ()> + Send + 'static,
    ) -> (SocketAddr, impl Future<Output = ()> + 'static) {
        let (addr, srv) = bind!(self, addr);
        let fut = srv.with_graceful_shutdown(signal).map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        });
        (addr, fut)
    }

    /// Setup this `Server` with a specific stream of incoming connections.
    ///
    /// This can be used for Unix Domain Sockets, or TLS, etc.
    ///
    /// Returns a `Future` that can be executed on any runtime.
    pub fn serve_incoming<I>(self, incoming: I) -> impl Future<Output = ()> + 'static
    where
        I: TryStream + Send + 'static,
        I::Ok: AsyncRead + AsyncWrite + Send + 'static + Unpin,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let incoming = incoming.map_ok(crate::transport::LiftIo);
        self.serve_incoming2(incoming)
    }

    async fn serve_incoming2<I>(self, incoming: I)
    where
        I: TryStream + Send,
        I::Ok: Transport + Send + 'static + Unpin,
        I::Error: Into<Box<dyn StdError + Send + Sync>>,
    {
        let service = into_service!(self.task);

        let srv = HyperServer::builder(hyper::server::accept::from_stream(incoming.into_stream()))
            .http1_pipeline_flush(self.pipeline)
            .serve(service)
            .await;

        if let Err(err) = srv {
            log::error!("server error: {}", err);
        }
    }

    // Generally shouldn't be used, as it can slow down non-pipelined responses.
    //
    // It's only real use is to make silly pipeline benchmarks look better.
    #[doc(hidden)]
    pub fn unstable_pipeline(mut self) -> Self {
        self.pipeline = true;
        self
    }

    /// Configure a server to use TLS.
    ///
    /// *This function requires the `"tls"` feature.*
    #[cfg(feature = "tls")]
    pub fn tls(self) -> TlsServer<T> {
        TlsServer {
            server: self,
            tls: TlsConfigBuilder::new(),
        }
    }

    #[cfg(all(unix, feature = "uds"))]
    pub fn uds(self) -> UnixDomainServer<T> {
        UnixDomainServer { server: self }
    }
}

// // ===== impl TlsServer =====

#[cfg(feature = "tls")]
impl<T> TlsServer<T>
where
    T: Task<Request, Error = Error> + Clone + Send + Sync + 'static,
    <T as Task<Request>>::Output: Reply, // <F::Future as TryFuture>::Ok: Reply,
                                         // <F::Future as TryFuture>::Error: IsReject
{
    // TLS config methods

    /// Specify the file path to read the private key.
    pub fn key_path(self, path: impl AsRef<Path>) -> Self {
        self.with_tls(|tls| tls.key_path(path))
    }

    /// Specify the file path to read the certificate.
    pub fn cert_path(self, path: impl AsRef<Path>) -> Self {
        self.with_tls(|tls| tls.cert_path(path))
    }

    /// Specify the in-memory contents of the private key.
    pub fn key(self, key: impl AsRef<[u8]>) -> Self {
        self.with_tls(|tls| tls.key(key.as_ref()))
    }

    /// Specify the in-memory contents of the certificate.
    pub fn cert(self, cert: impl AsRef<[u8]>) -> Self {
        self.with_tls(|tls| tls.cert(cert.as_ref()))
    }

    fn with_tls<Func>(self, func: Func) -> Self
    where
        Func: FnOnce(TlsConfigBuilder) -> TlsConfigBuilder,
    {
        let TlsServer { server, tls } = self;
        let tls = func(tls);
        TlsServer { server, tls }
    }

    // Server run methods

    /// Run this `TlsServer` forever on the current thread.
    ///
    /// *This function requires the `"tls"` feature.*
    pub async fn run(self, addr: impl Into<SocketAddr> + 'static) {
        let (addr, fut) = self.bind_ephemeral(addr);

        log::info!("listening on https://{}", addr);

        fut.await;
    }

    /// Bind to a socket address, returning a `Future` that can be
    /// executed on a runtime.
    ///
    /// *This function requires the `"tls"` feature.*
    pub async fn bind(self, addr: impl Into<SocketAddr> + 'static) {
        let (_, fut) = self.bind_ephemeral(addr);
        fut.await;
    }

    /// Bind to a possibly ephemeral socket address.
    ///
    /// Returns the bound address and a `Future` that can be executed on
    /// any runtime.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn bind_ephemeral(
        self,
        addr: impl Into<SocketAddr> + 'static,
    ) -> (SocketAddr, impl Future<Output = ()> + 'static) {
        let (addr, srv) = bind!(tls: self, addr);
        let srv = srv.map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        });

        (addr, srv)
    }

    /// Create a server with graceful shutdown signal.
    ///
    /// When the signal completes, the server will start the graceful shutdown
    /// process.
    ///
    /// *This function requires the `"tls"` feature.*
    pub fn bind_with_graceful_shutdown(
        self,
        addr: impl Into<SocketAddr> + 'static,
        signal: impl Future<Output = ()> + Send + 'static,
    ) -> (SocketAddr, impl Future<Output = ()> + 'static) {
        let (addr, srv) = bind!(tls: self, addr);

        let fut = srv.with_graceful_shutdown(signal).map(|result| {
            if let Err(err) = result {
                log::error!("server error: {}", err)
            }
        });
        (addr, fut)
    }
}

#[cfg(feature = "tls")]
impl<F> ::std::fmt::Debug for TlsServer<F>
where
    F: ::std::fmt::Debug,
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        f.debug_struct("TlsServer")
            .field("server", &self.server)
            .finish()
    }
}

#[cfg(all(unix, feature = "uds"))]
impl<T> UnixDomainServer<T>
where
    T: Task<Request, Error = Error> + Clone + Send + Sync + 'static,
    <T as Task<Request>>::Output: Reply,
{
    pub async fn run(self, path: impl AsRef<Path>) {
        match self.run2(path).await {
            Ok(_) => {}
            Err(err) => {
                log::error!("server error: {}", err);
            }
        };
    }

    async fn run2(
        self,
        path: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut listener = UnixListener::bind(path)?;
        let local_addr = listener.local_addr()?;
        let incoming = listener.incoming();
        let service = into_service!(self.server.task, Protocol::Unix, None);

        let srv = HyperServer::builder(hyper::server::accept::from_stream(incoming.into_stream()))
            .http1_pipeline_flush(self.server.pipeline)
            .serve(service)
            .await;

        if let Err(err) = srv {
            log::error!("server error: {}", err);
        }

        Ok(())
    }
}