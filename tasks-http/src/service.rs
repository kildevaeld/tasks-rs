use super::server::Protocol;
use super::transport::Transport;
use super::Response;
use super::{Error, Request};
use futures_core::ready;
use http::Method;
use hyper::service::Service;
use hyper::{Body, Request as HttpRequest, Response as HttpResponse, StatusCode};
use pin_project::pin_project;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tasks_core::{Rejection, Task};

pub fn service<F>(filter: F) -> TaskService<F>
where
    F: Task<Request>,
    // <F::Future as TryFuture>::Ok: Reply,
    // <F::Future as TryFuture>::Error: IsReject,
{
    TaskService::new(filter)
}

#[derive(Clone, Copy, Debug)]
pub struct TaskService<T> {
    task: T,
    pub(crate) local_addr: Option<SocketAddr>,
    pub(crate) protocol: Protocol,
}

impl<T> TaskService<T>
where
    T: Task<Request>,
{
    pub fn new(task: T) -> TaskService<T> {
        TaskService {
            task,
            local_addr: None,
            protocol: Protocol::Http,
        }
    }

    #[inline]
    pub(crate) fn call_with_addr(
        &self,
        req: HttpRequest<Body>,
        remote_addr: Option<SocketAddr>,
    ) -> ResponseFuture<T> {
        log::debug!("incoming {:?} on {:?}", remote_addr, req.uri());

        let mut http_res = HttpResponse::<Body>::new(Body::empty());
        *http_res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;

        let req = Request::from_http(req, self.protocol, self.local_addr);

        // //let method = req.method().clone();
        ResponseFuture::new(http_res, req.method().clone(), self.task.run(req))
    }
}

impl<T> Service<HttpRequest<Body>> for TaskService<T>
where
    T: Task<Request, Output = Response, Error = Error>,
{
    type Response = HttpResponse<Body>;
    type Error = Error;
    type Future = ResponseFuture<T>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: HttpRequest<Body>) -> Self::Future {
        self.call_with_addr(req, None)
    }
}

#[pin_project]
pub struct ResponseFuture<T>
where
    T: Task<Request>,
{
    res: Option<HttpResponse<Body>>,
    method: Option<Method>,
    #[pin]
    f: T::Future,
}

impl<T> ResponseFuture<T>
where
    T: Task<Request>,
{
    pub fn new(res: HttpResponse<Body>, method: Method, f: T::Future) -> ResponseFuture<T> {
        ResponseFuture {
            res: Some(res),
            method: Some(method),
            f,
        }
    }
}

impl<T> Future for ResponseFuture<T>
where
    T: Task<Request, Output = Response, Error = Error>,
{
    type Output = Result<HttpResponse<Body>, Error>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        let ret = ready!(this.f.poll(cx));
        let mut out = this.res.take().unwrap();
        let method = this.method.take().unwrap();

        match ret {
            Ok(resp) => {
                resp.write_back(&mut out, method);
            }
            Err(Rejection::Err(err)) => {
                err.response.write_back(&mut out, method);
            }
            Err(Rejection::Reject(req)) => {}
        };

        Poll::Ready(Ok(out))
    }
}

pub struct MakeTaskService<T> {
    task: T,
    local_address: Option<SocketAddr>,
    protocol: Protocol,
}

impl<T> MakeTaskService<T> {
    pub fn new(
        protocol: Protocol,
        local_address: Option<SocketAddr>,
        task: T,
    ) -> MakeTaskService<T> {
        MakeTaskService {
            task,
            protocol,
            local_address,
        }
    }
}

impl<'t, T, Ctx: Transport> Service<&'t Ctx> for MakeTaskService<T>
where
    T: Send + Clone + Task<Request, Output = Response, Error = Error>,
{
    type Response = TaskService<T>;
    type Error = Error;
    type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, cx: &'t Ctx) -> Self::Future {
        let remote = Transport::remote_addr(cx);
        let service = TaskService {
            task: self.task.clone(),
            protocol: self.protocol,
            local_addr: self.local_address,
        };

        futures::future::ok(service)

        // futures::future::ok(HyperService {
        //     inner: self.inner.clone(),
        //     protocol: self.protocol.clone(),
        //     local_address: self.local_address,
        // })
    }
}
