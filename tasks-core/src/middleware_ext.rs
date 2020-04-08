use super::{And, End, Middleware, Task};

pub trait MiddlewareExt<R>: Middleware<R> + Sized {
    fn and<M: Middleware<R, Output = Self::Output, Error = Self::Error>>(
        self,
        middleware: M,
    ) -> And<Self, M> {
        And::new(self, middleware)
    }

    fn end<T: Task<R, Output = Self::Output, Error = Self::Error>>(self, task: T) -> End<Self, T> {
        End::new(self, task)
    }
}

impl<R, T> MiddlewareExt<R> for T where T: Middleware<R> {}

#[cfg(test)]
mod test {

    use crate::*;

    #[test]
    fn test_middleware() {
        let m: MiddlewareFn<_, i32, i32, ()> = middleware!(|req, next| async move {
            let ret = next.run(req).await?;
            Ok(ret + 1)
        });

        let t = m.end(task!(|req| async move { Ok(req + 1) }));

        let ret = futures::executor::block_on(t.run(1));
        assert_eq!(ret, Ok(3));
    }
}
