use super::{Middleware, Stack, Task};

pub trait MiddlewareExt<R, TA>: Middleware<R, TA> + Sized
where
    TA: Task<R>,
{
    fn stack<M: Middleware<R, TA>>(self, middleware: M) -> Stack<Self, M> {
        Stack::new(self, middleware)
    }
}

impl<R, TA, T> MiddlewareExt<R, TA> for T
where
    T: Middleware<R, TA>,
    TA: Task<R>,
{
}

#[cfg(test)]
mod test {

    // use crate::*;

    // struct M;

    // impl<R, T> Middleware<R, T> for M {

    // }

    // #[test]
    // fn test_middleware() {
    //     let m: MiddlewareFn<_, i32, i32, ()> = middleware!(|req, next| async move {
    //         let ret = next.run(req).await?;
    //         Ok(ret + 1)
    //     });

    //     let t = m.end(task!(|req| async move { Ok(req + 1) }));

    //     let ret = futures::executor::block_on(t.run(1));
    //     assert_eq!(ret, Ok(3));
    // }
}
