use super::Protocol;
use headers::{Cookie, HeaderMap, HeaderMapExt};
use http::Extensions;
use hyper::{header, Body, Method, Request as HttpRequest, Version};
use std::net::SocketAddr;
use url::Url;

pub struct Request {
    inner: HttpRequest<Body>,
    url: Url,
}

impl Request {
    pub(crate) fn from_http(
        req: HttpRequest<Body>,
        protocol: Protocol,
        local_addr: Option<SocketAddr>,
    ) -> Request {
        let uri = req.uri();
        let headers = req.headers();
        let version = req.version();
        let url = {
            let path = uri.path();

            let mut socket_ip = String::new();
            let (host, port) = if let Some(host) = uri.host() {
                (host, uri.port().and_then(|p| Some(p.as_u16())))
            } else if let Some(host) = headers.get(header::HOST).and_then(|h| h.to_str().ok()) {
                let mut parts = host.split(':');
                let hostname = parts.next().unwrap();
                let port = parts.next().and_then(|p| p.parse::<u16>().ok());
                (hostname, port)
            } else if version < Version::HTTP_11 {
                if let Some(local_addr) = local_addr {
                    match local_addr {
                        SocketAddr::V4(addr4) => socket_ip.push_str(&format!("{}", addr4.ip())),
                        SocketAddr::V6(addr6) => socket_ip.push_str(&format!("[{}]", addr6.ip())),
                    }
                    (socket_ip.as_ref(), Some(local_addr.port()))
                } else {
                    panic!("No fallback host specified");
                    //return Err("No fallback host specified".into());
                }
            } else {
                panic!("No host specified in request");
                //return Err("No host specified in request".into());
            };

            let url_string = if let Some(port) = port {
                match uri.query() {
                    Some(q) => format!("{}://{}:{}{}?{}", protocol, host, port, path, q),
                    None => format!("{}://{}:{}{}", protocol, host, port, path),
                }
            } else {
                match uri.query() {
                    Some(q) => format!("{}://{}{}?{}", protocol, host, path, q),
                    None => format!("{}://{}{}", protocol, host, path),
                }
            };

            match Url::parse(&url_string) {
                Ok(url) => url,
                Err(e) => panic!("Couldn't parse requested URL: {}", e),
            }
        };

        Request { inner: req, url }
    }

    pub fn method(&self) -> &Method {
        self.inner.method()
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn url_mut(&mut self) -> &mut Url {
        &mut self.url
    }

    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.inner.headers_mut()
    }

    pub fn extensions(&self) -> &Extensions {
        self.inner.extensions()
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        self.inner.extensions_mut()
    }

    pub fn cookie(&self) -> Option<Cookie> {
        self.headers().typed_get()
    }
}
