use super::either::Either;
use super::pipe::Pipe;
use super::task::{IntoTask, Task};
use std::sync::Arc;

pub trait TaskExt<I>: Task<I> + Sized {
    fn pipe<S: IntoTask<Self::Output, Error = Self::Error>>(self, service: S) -> Pipe<Self, S::Task> {
        Pipe {
            s1: self,
            s2: Arc::new(service.into_task()),
        }
    }

    fn or<S: IntoTask<I, Output = Self::Output, Error = Self::Error>>(
        self,
        service: S,
    ) -> Either<Self, S::Task>
   
    {
        Either::new(self, service.into_task())
    }
}

impl<T, I> TaskExt<I> for T where T: Task<I> {}

#[cfg(test)]
mod tests {
    use super::super::error::TaskError;
    use super::super::task::*;
    use super::*;

    use super::super::*;

    #[test]
    fn test_task_pipe() {
        let s = task_fn!(|_input: &str| futures_util::future::ok::<_, ()>(2000i32))
            .pipe(task_fn!(|input: i32| futures_util::future::ok(input + 2)));

        let ret = futures_executor::block_on(s.exec("Hello, World!"));
        assert_eq!(ret, Ok(2002));
    }

    #[test]
    fn test_conditional_task_or() {
        let s = task_fn!(
            |input: i32| futures_util::future::ok::<_, TaskError>(input + 1),
            |&input| input == 1
        )
        .or(task_fn!(
            |input: i32| futures_util::future::ok::<_, TaskError>(input + 2),
            |&input| input == 2
        )
        .pipe(task_fn!(|input| {
            futures_util::future::ok::<_, TaskError>(input + 1)
        })));

        assert_eq!(futures_executor::block_on(s.exec(1)), Ok(2));
        assert_eq!(futures_executor::block_on(s.exec(2)), Ok(5));
        assert_eq!(
            futures_executor::block_on(s.exec(3)),
            Err(TaskError::InvalidRequest)
        );
    }
}
