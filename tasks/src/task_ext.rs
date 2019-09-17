use super::chain::TaskChain;
use super::pipe::Pipe;
use super::task::{ConditionalTask, IntoConditionalTask, IntoTask, Task};
use std::sync::Arc;

pub trait TaskExt: Task + Sized {
    fn pipe<S: IntoTask<Error = Self::Error>>(self, service: S) -> Pipe<Self, S::Task> {
        Pipe {
            s1: self,
            s2: Arc::new(service.into_task()),
        }
    }
}

impl<T> TaskExt for T where T: Task {}

pub trait ConditionalTaskExt: ConditionalTask + Sized {
    fn or<
        S: IntoConditionalTask<Input = Self::Input, Output = Self::Output, Error = Self::Error>,
    >(
        self,
        service: S,
    ) -> TaskChain<Self, S::Task> {
        TaskChain::new(self, service.into_task())
    }
}

impl<T> ConditionalTaskExt for T where T: ConditionalTask {}



#[cfg(test)]
mod tests {
    use super::super::error::TaskError;
    use super::super::task::*;
    use super::*;

    #[test]
    fn test_task_pipe() {
        let s = task_fn(|_input: &str| futures::future::ok::<_, ()>(2000i32))
            .pipe(task_fn(|input: i32| futures::future::ok(input + 2)));

        let ret = futures::executor::block_on(s.exec("Hello, World!"));
        assert_eq!(ret, Ok(2002));
    }

    #[test]
    fn test_conditional_task_or() {
        let s = conditional_task_fn(
            |input: i32| futures::future::ok::<_, TaskError>(input + 1),
            |&input| input == 1,
        )
        .or(conditional_task_fn(
            |input: i32| futures::future::ok::<_, TaskError>(input + 2),
            |&input| input == 2,
        ).pipe(task_fn(|input| {
            futures::future::ok::<_, TaskError>(input + 1)
        })));

        assert_eq!(futures::executor::block_on(s.exec(1)), Ok(2));
        assert_eq!(futures::executor::block_on(s.exec(2)), Ok(5));
        assert_eq!(futures::executor::block_on(s.exec(3)), Err(TaskError::InvalidRequest));
    }

}
