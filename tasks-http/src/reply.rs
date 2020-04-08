use super::Response;

pub trait Reply {
    fn into_response(self) -> Response;
}

impl Reply for Response {
    fn into_response(self) -> Response {
        self
    }
}
