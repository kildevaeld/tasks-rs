use super::{reply::Reply, Error, Request};
use tasks_core::{task, Rejection, Task, TaskExt};

pub fn mount<S: AsRef<str>, T>(
    path: S,
    task: T,
) -> impl Task<Request, Output = T::Output, Error = T::Error> + Clone
where
    T: Task<Request, Error = Error> + Clone + Send + Sync + 'static,
    T::Output: Reply + Send,
{
    crate::filters::mount(path)
        .map(move || task.clone())
        .then(task!(|req: (Request, (T,))| async move {
            let (req, (task,)) = req;
            match task.run(req).await {
                Ok(ret) => Ok(ret),
                Err(Rejection::Err(err)) => Err(Rejection::Err(err)),
                Err(Rejection::Reject(req)) => Err(Rejection::Reject((req, (task,)))),
            }
        }))
}
