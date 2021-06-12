use super::{Rejection, Service, ServiceFn};

pub fn pass<R: Send + 'static, E: Send>() -> impl Service<R, Output = (R, ()), Error = E> + Copy {
    ServiceFn::new(|req: R| async move { Result::<_, Rejection<R, E>>::Ok((req, ())) })
}
