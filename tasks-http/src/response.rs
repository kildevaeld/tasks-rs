use hyper::{header, Body, HeaderMap, Method, Response as HttpResponse, StatusCode};
use modifier::{Modifier, Set};
use std::fmt;

pub struct Response {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Option<Body>,
}

impl Response {
    pub fn new() -> Response {
        Response {
            status: StatusCode::NOT_FOUND,
            headers: HeaderMap::default(),
            body: None,
        }
    }

    pub fn with<M: Modifier<Response>>(m: M) -> Response {
        Response::new().set(m)
    }

    pub(crate) fn write_back(self, resp: &mut HttpResponse<Body>, method: Method) {
        *resp.headers_mut() = self.headers;
        *resp.status_mut() = self.status;

        match (self.body, method) {
            (Some(body), _) => {
                let content_type = resp.headers().get(header::CONTENT_TYPE).map_or_else(
                    || header::HeaderValue::from_static("text/plain"),
                    |cx| cx.clone(),
                );
                resp.headers_mut()
                    .insert(header::CONTENT_TYPE, content_type);
                *resp.body_mut() = body;
            }
            (None, Method::HEAD) => {}
            (None, _) => {
                resp.headers_mut().insert(
                    header::CONTENT_LENGTH,
                    header::HeaderValue::from_static("0"),
                );
            }
        };
    }
}

impl Set for Response {}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "HTTP/1.1 {}\n{:?}", self.status, self.headers)
    }
}
