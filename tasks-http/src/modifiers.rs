use super::{Request, Response};
pub use headers::{
    AcceptRanges, CacheControl, Connection, ContentLength, ContentRange, ContentType, ETag,
    Expires, HeaderMapExt, LastModified, Location, SecWebsocketAccept, Upgrade, UserAgent,
};
use http::{header, StatusCode};
use hyper::Body;
use modifier::Modifier;
use url::Url;

impl Modifier<Response> for Body {
    #[inline]
    fn modify(self, res: &mut Response) {
        res.body = Some(self);
    }
}

impl Modifier<Response> for String {
    #[inline]
    fn modify(self, res: &mut Response) {
        self.into_bytes().modify(res);
    }
}

impl Modifier<Response> for Vec<u8> {
    #[inline]
    fn modify(self, res: &mut Response) {
        res.headers
            .insert(header::CONTENT_LENGTH, (self.len() as u64).into());
        res.body = Some(Body::from(self));
    }
}

impl<'a> Modifier<Response> for &'a str {
    #[inline]
    fn modify(self, res: &mut Response) {
        self.to_owned().modify(res);
    }
}

impl<'a> Modifier<Response> for &'a [u8] {
    #[inline]
    fn modify(self, res: &mut Response) {
        self.to_vec().modify(res);
    }
}

impl Modifier<Response> for StatusCode {
    fn modify(self, res: &mut Response) {
        res.status = self;
    }
}

/// A modifier for changing headers on requests and responses.
#[derive(Clone)]
pub struct Header<H>(pub H, pub header::HeaderValue);

impl<H> Modifier<Response> for Header<H>
where
    H: header::IntoHeaderName,
{
    fn modify(self, res: &mut Response) {
        res.headers.insert(self.0, self.1);
    }
}

macro_rules! typed_header_impl {
    ($header: ty) => {
        impl Modifier<Response> for $header {
            fn modify(self, res: &mut Response) {
                res.headers.typed_insert(self)
            }
        }
    };
}

typed_header_impl!(ContentType);
typed_header_impl!(ContentLength);
typed_header_impl!(CacheControl);
typed_header_impl!(ETag);
typed_header_impl!(Expires);
typed_header_impl!(Upgrade);
typed_header_impl!(Connection);
typed_header_impl!(SecWebsocketAccept);
typed_header_impl!(ContentRange);
typed_header_impl!(AcceptRanges);
typed_header_impl!(LastModified);
/// A modifier for creating redirect responses.
pub struct Redirect(pub Url);

impl Modifier<Response> for Redirect {
    fn modify(self, res: &mut Response) {
        let Redirect(url) = self;
        // Url should always be parsable to a valid HeaderValue, so unwrap should be safe here.
        res.headers
            .insert(header::LOCATION, url.to_string().parse().unwrap());
    }
}

/// A modifier for creating redirect responses.
pub struct RedirectRaw(pub String);

impl Modifier<Response> for RedirectRaw {
    fn modify(self, res: &mut Response) {
        let RedirectRaw(path) = self;
        res.headers.insert(header::LOCATION, path.parse().unwrap());
    }
}
