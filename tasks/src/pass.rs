use crate::{Result, Task, TaskFn};

pub fn pass<R: Send + 'static, E: Send>() -> impl Task<R, Output = R, Error = E> + Copy {
    TaskFn::new(|req: R| async move { Result::<R, R, E>::Ok(req) })
}
