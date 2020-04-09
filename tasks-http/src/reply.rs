use super::modifiers::Header;
use super::Response;
use http::{HeaderValue, StatusCode};
use modifier::Set;

pub trait Reply {
    fn into_response(self) -> Response;
}

impl Reply for Response {
    fn into_response(self) -> Response {
        self
    }
}

pub struct Html<'a> {
    body: &'a str,
}

impl<'a> Reply for Html<'a> {
    fn into_response(self) -> Response {
        Response::with(StatusCode::OK).set(self.body).set(Header(
            "Content-Type",
            HeaderValue::from_static("text/html"),
        ))
    }
}

pub fn html<'a>(body: &'a str) -> Html<'a> {
    Html { body }
}
