use super::task::{ConditionalSyncTask, SyncTask};
use crate::error::TaskError;

pub struct Either<S1, S2> {
    s1: S1,
    s2: S2,
}

impl<S1, S2> Either<S1, S2> {
    pub fn new(s1: S1, s2: S2) -> Either<S1, S2> {
        Either { s1, s2 }
    }
}

impl<S1, S2> SyncTask for Either<S1, S2>
where
    S1: ConditionalSyncTask,
    <S1 as SyncTask>::Output: Send + 'static,
    <S1 as SyncTask>::Error: Send + 'static + From<TaskError>,
    S2: ConditionalSyncTask<
        Input = <S1 as SyncTask>::Input,
        Output = <S1 as SyncTask>::Output,
        Error = <S1 as SyncTask>::Error,
    >,
{
    type Input = S1::Input;
    type Output = S1::Output;
    type Error = S1::Error;

    fn exec(&self, ctx: Self::Input) -> Result<Self::Output, Self::Error> {
        if self.s1.can_exec(&ctx) {
            self.s1.exec(ctx)
        } else if self.s2.can_exec(&ctx) {
            self.s2.exec(ctx)
        } else {
            Err(Self::Error::from(TaskError::InvalidRequest))
        }
    }
}

impl<S1, S2> ConditionalSyncTask for Either<S1, S2>
where
    S1: ConditionalSyncTask,
    <S1 as SyncTask>::Output: Send + 'static,
    <S1 as SyncTask>::Error: Send + 'static + From<TaskError>,
    S2: ConditionalSyncTask<
        Input = <S1 as SyncTask>::Input,
        Output = <S1 as SyncTask>::Output,
        Error = <S1 as SyncTask>::Error,
    >,
{
    #[inline]
    fn can_exec(&self, ctx: &Self::Input) -> bool {
        self.s1.can_exec(ctx) || self.s2.can_exec(ctx)
    }
}
