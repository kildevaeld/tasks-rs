use super::{Asset, AssetRequest, Extensions};
use tasks::{Either, One};

pub trait Reply {
    fn into_response(self) -> AssetResponse;
}

impl<T> Reply for One<T>
where
    T: Reply,
{
    fn into_response(self) -> AssetResponse {
        self.0.into_response()
    }
}

impl<T> Reply for (AssetRequest, One<T>)
where
    T: Reply,
{
    fn into_response(self) -> AssetResponse {
        self.1.into_response()
    }
}

impl Reply for AssetResponse {
    #[inline(always)]
    fn into_response(self) -> AssetResponse {
        self
    }
}

impl<T, U> Reply for Either<T, U>
where
    T: Reply,
    U: Reply,
{
    #[inline(always)]
    fn into_response(self) -> AssetResponse {
        match self {
            Either::A(a) => a.into_response(),
            Either::B(b) => b.into_response(),
        }
    }
}

pub struct AssetResponse {
    pub(crate) request: AssetRequest,
    pub(crate) node: Asset,
}

impl AssetResponse {
    pub fn node(&self) -> &Asset {
        &self.node
    }

    pub fn into_node(self) -> Asset {
        self.node
    }
}
