use super::{Error, Request, Response};
use async_compression::flate2::Compression;
use async_compression::stream::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use futures::StreamExt;
use headers::{ContentEncoding, ContentLength, ContentType, HeaderMapExt, HeaderValue};
use hyper::{header, Body};
use std::future::Future;
use std::pin::Pin;
use tasks_core::{Middleware, Next, Rejection};

#[derive(Clone, Copy, Debug)]
pub struct Compress;

pub fn compress() -> Compress {
    Compress
}

#[derive(Copy, Clone, PartialEq)]
enum Type {
    Gzip,
    Deflate,
    Brotli,
}

fn to_type(types: &str) -> Vec<Type> {
    types
        .split(",")
        .map(|m| m.trim())
        .filter_map(|m| match m {
            "gzip" => Some(Type::Gzip),
            "deflate" => Some(Type::Deflate),
            "br" => Some(Type::Brotli),
            _ => None,
        })
        .collect::<Vec<_>>()
}

fn to_compression(ty: Type, resp: &mut Response) {
    if resp.body.is_none() {
        return;
    }

    let body = resp.body.take().unwrap();
    let (header, body) = match ty {
        Type::Gzip => {
            let body = GzipEncoder::new(
                body.map(|m| match m {
                    Ok(s) => Ok(s),
                    Err(e) => Err(std::io::ErrorKind::Interrupted.into()),
                }),
                Compression::fast(),
            );
            (HeaderValue::from_static("gzip"), Body::wrap_stream(body))
        }
        Type::Deflate => {
            let body = DeflateEncoder::new(
                body.map(|m| match m {
                    Ok(s) => Ok(s),
                    Err(e) => Err(std::io::ErrorKind::Interrupted.into()),
                }),
                Compression::fast(),
            );
            (HeaderValue::from_static("deflate"), Body::wrap_stream(body))
        }
        Type::Brotli => {
            let body = BrotliEncoder::new(
                body.map(|m| match m {
                    Ok(s) => Ok(s),
                    Err(e) => Err(std::io::ErrorKind::Interrupted.into()),
                }),
                9,
            );
            (HeaderValue::from_static("br"), Body::wrap_stream(body))
        }
    };

    resp.headers.insert("Content-Encoding", header);
    resp.headers.remove("Content-Length");
    resp.body = Some(body);
}

impl Middleware<Request> for Compress {
    type Output = Response;
    type Error = Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Output, Rejection<Request, Self::Error>>> + Send>>;

    fn run<N: Clone + 'static + Next<Request, Output = Self::Output, Error = Self::Error>>(
        &self,
        req: Request,
        next: N,
    ) -> Self::Future {
        let accept_ec = req
            .headers()
            .get(header::ACCEPT_ENCODING)
            .map(|m| m.to_owned());
        let future = async move {
            let mut resp = next.run(req).await?;

            if resp.body.is_none() {
                return Ok(resp);
            }

            if accept_ec.is_none() {
                return Ok(resp);
            }
            let accept_ec = accept_ec.unwrap();

            let accepted_types = to_type(accept_ec.to_str().unwrap());

            if accepted_types.is_empty() {
                return Ok(resp);
            }

            let first = accepted_types.first().unwrap();

            to_compression(*first, &mut resp);

            Ok(resp)
        };

        Box::pin(future)
    }
}
