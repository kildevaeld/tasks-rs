use mime::Mime;
use multipart::server::Multipart;
use std::io::{Cursor, Read};

pub struct Form {}

pub struct FormData {
    nner: Multipart<Cursor<::bytes::Bytes>>,
}

pub struct Part {
    name: String,
    filename: Option<String>,
    content_type: Option<String>,
    data: Option<Vec<u8>>,
}
