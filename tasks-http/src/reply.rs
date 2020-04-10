use super::modifiers::{ContentLength, ContentType};
use super::Response;
use http::StatusCode;
use modifier::Set;
#[cfg(feature = "json")]
use serde::Serialize;

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

#[cfg(feature = "json")]
pub struct Json<S: Serialize> {
    value: S,
    pretty: bool,
}

#[cfg(feature = "json")]
impl<S: Serialize> Json<S> {
    fn pretty(self) -> Json<S> {
        Json {
            value: self.value,
            pretty: self.pretty,
        }
    }
}

#[cfg(feature = "json")]
impl<S: Serialize> Reply for Json<S> {
    #[inline(always)]
    fn into_response(self) -> Response {
        let data = if self.pretty {
            serde_json::to_string_pretty(&self.value)
        } else {
            serde_json::to_string(&self.value)
        };

        let data = match data {
            Ok(data) => data,
            Err(_) => unimplemented!("Not implemented"),
        };

        let len = data.len();

        Response::with(StatusCode::OK)
            .set(data)
            .set(ContentType::json())
            .set(ContentLength(len as u64))
    }
}

#[cfg(feature = "json")]
pub fn json<S: Serialize>(value: S) -> Json<S> {
    Json {
        value,
        pretty: false,
    }
}
