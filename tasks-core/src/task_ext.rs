use super::filter::filter_fn_one;
use super::{Map, Or, Task};
pub trait TaskExt<R>: Task<R> + Sized {
    fn or<T: Task<R, Output = Self::Output, Error = Self::Error>>(self, task: T) -> Or<Self, T> {
        Or::new(self, task)
    }

    fn then<T: Task<Self::Output>>(self, task: T) -> Map<Self, T> {
        Map::new(self, task)
    }
}

impl<R, T> TaskExt<R> for T where T: Task<R> {}

#[cfg(test)]
mod test {

    use crate::*;

    #[test]
    fn test_task() {
        let t = task!(|req: i32| async move { Result::<_, Rejection<i32, ()>>::Ok(req + 1) });

        let ret = futures::executor::block_on(t.run(1));
        assert_eq!(ret, Ok(2));
    }

    #[test]
    fn test_reject() {
        let t: TaskFn<_, _, i32, ()> = task!(|req: i32| async move { reject!(req) });

        let ret = futures::executor::block_on(t.run(1));
        assert_eq!(ret, Err(Rejection::Reject(1)));
    }

    #[test]
    fn test_or() {
        let t = task!(|req: i32| async move {
            if req != 1 {
                reject!(req);
            } else {
                Result::<_, Rejection<_, ()>>::Ok(req + 1)
            }
        })
        .or(task!(|req| async move {
            if req != 2 {
                reject!(req);
            }
            Result::<_, Rejection<_, ()>>::Ok(req + 2)
        }))
        .or(task!(|req| async move { Ok(req + 3) }));

        let ret = futures::executor::block_on(t.run(1));
        assert_eq!(ret, Ok(2));

        let ret = futures::executor::block_on(t.run(2));
        assert_eq!(ret, Ok(4));

        let ret = futures::executor::block_on(t.run(3));
        assert_eq!(ret, Ok(6));

        let ret = futures::executor::block_on(t.run(6));
        assert_eq!(ret, Ok(9));
    }

    #[test]
    fn test_then() {
        let t = task!(|req: i32| async move {
            Result::<_, Rejection<i32, ()>>::Ok(format!("{}", req + 1))
        })
        .then(task!(|req: String| async move {
            let p: i32 = req.parse().unwrap();
            Result::<_, Rejection<_, ()>>::Ok(p + 1)
        }))
        .then(task!(|req: i32| async move {
            Result::<_, Rejection<i32, ()>>::Ok(req + 1)
        }));
        // .then(task!(|req: i32| async move {
        //     Result::<_, Rejection<i32, ()>>::Ok(format!("{}", req))
        // }));

        let ret = futures::executor::block_on(t.run(1));
        assert_eq!(ret, Ok(3));
    }
}
