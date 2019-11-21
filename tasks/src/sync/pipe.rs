use super::task::{SyncTask};

pub struct SyncPipe<S1, S2> {
    pub(crate) s1: S1,
    pub(crate) s2: S2,
}

impl<S1, S2> SyncTask for SyncPipe<S1, S2>
where
    S1: SyncTask,
    S2: SyncTask<Input = <S1 as SyncTask>::Output, Error = <S1 as SyncTask>::Error>,
{
    type Input = S1::Input;
    type Error = S1::Error;
    type Output = S2::Output;

    fn exec(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
        match self.s1.exec(input) {
            Ok(i) => self.s2.exec(i),
            Err(e) => Err(e)
        }
    }

    fn can_exec(&self, input: &Self::Input) ->bool {
        self.s1.can_exec(input)
    }


}


// impl<S1, S2> ConditionalSyncTask for SyncPipe<S1, S2>
// where
//     S1: ConditionalSyncTask,
//     S2: SyncTask<Input = <S1 as SyncTask>::Output, Error = <S1 as SyncTask>::Error>
// {
    
//     fn can_exec(&self, input: &Self::Input) ->bool {
//         self.s1.can_exec(input)
//     }

// }

