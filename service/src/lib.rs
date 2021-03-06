#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "alloc")]
mod boxed;
mod either;
mod generic;
mod macros;
mod map;
mod middleware;
mod rejection;
mod service;
mod service_ext;
mod util;
pub mod vec;

pub mod and;
pub mod and_then;
pub mod and_then_reject;
pub mod err_into;
pub mod flatten;
pub mod map_err;
pub mod or_else;
pub mod then;
pub mod unify;
pub mod unpack;

pub use self::{
    either::*,
    generic::{one, Combine, Extract, Func, HList, One, Tuple},
    map::*,
    middleware::*,
    rejection::*,
    service::*,
    service_ext::*,
    util::*,
};
#[cfg(feature = "alloc")]
pub use boxed::*;

pub mod prelude {
    pub use super::{service, ServiceExt, ServiceExtract};
}

#[cfg(test)]
mod test {
    use super::*;

    struct Param {}

    struct Error {}

    #[test]
    fn test_or() {
        let service = ServiceFn::<_, _, _, Param>::new(|_: Param| async move { Ok(1000) });
        let service2 = ServiceFn::<_, _, _, Param>::new(|_: Param| async move { Ok(1000) });

        service.or_else(service2).unify();

        let service =
            ServiceFn::<_, _, _, Param>::new(|test: Param| async move { Ok((test, (1000,))) });
        let service2 =
            ServiceFn::<_, _, _, Param>::new(|test: Param| async move { Ok((test, (1000,))) });

        service.or_else(service2).unify();
    }

    #[test]
    fn test_pass() {
        let test = pass::<Param, Error>().map(|| 200).unpack();
    }

    #[test]
    fn test_flatten() {
        let test = pass::<Param, Error>()
            .map(|| 200)
            .and(pass::<Param, Error>().map(|| false));
    }
}
