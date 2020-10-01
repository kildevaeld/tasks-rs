pub type Result<R, O, E> = std::result::Result<O, Rejection<R, E>>;

#[derive(Debug, PartialEq)]
pub enum Rejection<R, E> {
    Err(E),
    Reject(R, Option<E>),
}

impl<R, E> From<E> for Rejection<R, E> {
    fn from(error: E) -> Self {
        Rejection::Err(error)
    }
}
