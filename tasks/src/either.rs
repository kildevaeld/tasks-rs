use super::error::TaskError;
use super::task::{Task};
use super::utils::{OneOf3Future, Promise3};
use futures_util::future::Ready;

pub struct Either<S1, S2> {
    s1: S1,
    s2: S2,
}

impl<S1, S2> Either<S1, S2> {
    pub fn new(s1: S1, s2: S2) -> Either<S1, S2> {
        Either { s1, s2 }
    }
}

impl<S1, S2, I> Task<I> for Either<S1, S2>
where
    S1: Task<I>,
    <S1 as Task<I>>::Output: Send + 'static,
    <S1 as Task<I>>::Error: Send + 'static + From<TaskError>,
    S2: Task<
        I,
        Output = <S1 as Task<I>>::Output,
        Error = <S1 as Task<I>>::Error,
    >,
{
    
    type Output = S1::Output;
    type Error = S1::Error;

    type Future = OneOf3Future<
        S1::Future,
        S2::Future,
        Ready<Result<Self::Output, Self::Error>>,
        Result<Self::Output, Self::Error>
    >;

    fn exec(&self, ctx: I) -> Self::Future {
        let fut = if self.s1.can_exec(&ctx) {
            Promise3::First(self.s1.exec(ctx))
        } else if self.s2.can_exec(&ctx) {
            Promise3::Second(self.s2.exec(ctx))
        } else {
            Promise3::Third(futures_util::future::err(Self::Error::from(
                TaskError::InvalidRequest,
            )))
           
        };
        OneOf3Future::new(fut)
    }

    #[inline]
    fn can_exec(&self, ctx: &I) -> bool {
        self.s1.can_exec(ctx) || self.s2.can_exec(ctx)
    }

    
}
