// use super::filter::filter_fn_one;
use super::{And, AndThen, Combine, Extract, Func, Map, Or, Pipe, Task, Tuple, Unroll};
use futures_core::TryFuture;

pub trait TaskExt<R>: Task<R> + Sized {
    fn or<T: Task<R, Error = Self::Error>>(self, task: T) -> Or<Self, T> {
        Or::new(self, task)
    }

    fn then<T: Task<Self::Output>>(self, task: T) -> Pipe<Self, T> {
        Pipe::new(self, task)
    }

    // Filters
    fn and<F>(self, other: F) -> And<Self, F>
    where
        Self: Sized,
        Self::Output: Extract<R>,
        <<Self::Output as Extract<R>>::Extract as Tuple>::HList:
            Combine<<<F::Output as Extract<R>>::Extract as Tuple>::HList>,
        F: Task<R> + Clone,
        F::Output: Extract<R>,
        //F::Error: CombineRejection<Self::Error>,
    {
        And {
            first: self,
            second: other,
        }
    }

    fn map<F>(self, fun: F) -> Map<Self, F>
    where
        Self: Sized,
        Self::Output: Extract<R>,
        F: Func<<Self::Output as Extract<R>>::Extract> + Clone,
    {
        Map {
            filter: self,
            callback: fun,
        }
    }

    fn unroll(self) -> Unroll<Self>
    where
        Self: Sized,
        Self::Output: Extract<R>,
    {
        Unroll(self)
    }

    fn and_then<F>(self, fun: F) -> AndThen<Self, F>
    where
        Self: Sized,
        Self::Output: Extract<R>,
        F: Func<<Self::Output as Extract<R>>::Extract> + Clone,
        F::Output: TryFuture + Send,
        <F::Output as TryFuture>::Error: Into<Self::Error>, //CombineRejection<Self::Error>,
    {
        AndThen {
            filter: self,
            callback: fun,
        }
    }
}

impl<R, T> TaskExt<R> for T where T: Task<R> {}

#[cfg(test)]
mod test {

    use crate::*;

    #[derive(Debug)]
    struct Person {
        name: String,
        age: u8,
    }

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
        assert_eq!(ret, Ok(4));
    }

    #[test]
    fn test_map() {
        let m = task!(|req: Person| async move {
            let name = req.name.clone();
            Result::<_, Rejection<_, ()>>::Ok((req, (name,)))
        })
        // .and(task!(|req: Person| async move {
        //     let age = req.age;
        //     Ok((req, (age,)))
        // }))
        .and(filter_fn_one(|req: &mut Person| {
            let age = req.age;
            async move { Ok(age) }
        }))
        .map(|name, age| format!("name: {}, age: {}", name, age))
        .unroll()
        //.//untuple_one();
        .then(task!(|ret: (String,)| { futures::future::ok(ret.0) }));

        let ret = futures::executor::block_on(m.run(Person {
            name: "Rasmus".to_owned(),
            age: 36,
        }));

        assert!(ret.is_ok());
        let ret = ret.unwrap();
        assert_eq!(String::from("name: Rasmus, age: 36"), ret);
    }
}
