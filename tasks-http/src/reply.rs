use super::modifiers::{ContentLength, ContentType};
use super::Response;
use http::StatusCode;
use modifier::Set;

pub trait Reply {
    fn into_response(self) -> Response;
}

impl Reply for Response {
    #[inline(always)]
    fn into_response(self) -> Response {
        self
    }
}

pub struct Html<'a> {
    body: &'a str,
}

impl<'a> Reply for Html<'a> {
    #[inline(always)]
    fn into_response(self) -> Response {
        Response::with(StatusCode::OK)
            .set(self.body)
            .set(ContentType::html())
            .set(ContentLength(self.body.len() as u64))
    }
}

pub fn html<'a>(body: &'a str) -> Html<'a> {
    Html { body }
}

pub struct Text<'a> {
    body: &'a str,
}

impl<'a> Reply for Text<'a> {
    #[inline(always)]
    fn into_response(self) -> Response {
        Response::with(StatusCode::OK)
            .set(self.body)
            .set(ContentType::text())
            .set(ContentLength(self.body.len() as u64))
    }
}

pub fn text<'a>(body: &'a str) -> Text<'a> {
    Text { body }
}

impl<'a> Reply for &'a str {
    #[inline(always)]
    fn into_response(self) -> Response {
        let len = self.len();
        Response::with(StatusCode::OK)
            .set(self)
            .set(ContentType::text())
            .set(ContentLength(len as u64))
    }
}
