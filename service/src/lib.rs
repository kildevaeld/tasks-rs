pub mod and;
mod either;
pub mod err_into;
mod generic;
mod map;
pub mod map_err;
pub mod or;
mod rejection;
mod service;
mod service_ext;
pub mod unify;

pub use self::{
    either::*,
    generic::{one, Combine, Extract, Func, One, Tuple},
    map::*,
    rejection::*,
    service::*,
    service_ext::*,
};

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_or() {
        let service = ServiceFn::<_, _, _, String>::new(|test: String| async move { Ok(1000) });
        let service2 = ServiceFn::<_, _, _, String>::new(|test: String| async move { Ok(1000) });

        service.or(service2).unify();

        let service =
            ServiceFn::<_, _, _, String>::new(|test: String| async move { Ok((test, (1000,))) });
        let service2 =
            ServiceFn::<_, _, _, String>::new(|test: String| async move { Ok((test, (1000,))) });

        let service = service.or(service2).unify();
    }
}
