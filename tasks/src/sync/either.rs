use super::task::{SyncTask};
use crate::error::TaskError;

pub struct EitherSync<S1, S2> {
    s1: S1,
    s2: S2,
}

impl<S1, S2> EitherSync<S1, S2> {
    pub fn new(s1: S1, s2: S2) -> EitherSync<S1, S2> {
        EitherSync { s1, s2 }
    }
}

impl<S1, S2, I> SyncTask<I> for EitherSync<S1, S2>
where
    S1: SyncTask<I>,
    <S1 as SyncTask<I>>::Output: Send + 'static,
    <S1 as SyncTask<I>>::Error: Send + 'static + From<TaskError>,
    S2: SyncTask<
        I,
        Output = <S1 as SyncTask<I>>::Output,
        Error = <S1 as SyncTask<I>>::Error,
    >,
{
    type Output = S1::Output;
    type Error = S1::Error;

    fn exec(&self, ctx: I) -> Result<Self::Output, Self::Error> {
        if self.s1.can_exec(&ctx) {
            self.s1.exec(ctx)
        } else if self.s2.can_exec(&ctx) {
            self.s2.exec(ctx)
        } else {
            Err(Self::Error::from(TaskError::InvalidRequest))
        }
    }

    #[inline]
    fn can_exec(&self, ctx: &I) -> bool {
        self.s1.can_exec(ctx) || self.s2.can_exec(ctx)
    }
}

// impl<S1, S2> ConditionalSyncTask for EitherSync<S1, S2>
// where
//     S1: ConditionalSyncTask,
//     <S1 as SyncTask>::Output: Send + 'static,
//     <S1 as SyncTask>::Error: Send + 'static + From<TaskError>,
//     S2: ConditionalSyncTask<
//         Input = <S1 as SyncTask>::Input,
//         Output = <S1 as SyncTask>::Output,
//         Error = <S1 as SyncTask>::Error,
//     >,
// {
//     #[inline]
//     fn can_exec(&self, ctx: &Self::Input) -> bool {
//         self.s1.can_exec(ctx) || self.s2.can_exec(ctx)
//     }
// }
