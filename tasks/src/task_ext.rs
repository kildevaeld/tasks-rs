use super::{
    boxtask, And, AndThen, BoxTask, Combine, Either, Extract, FilterPipe, Func, Map, MapErr,
    Middleware, Or, Pipe, Reject, Rejection, Task, Tuple, Unify, Unify2, Unroll,
};
use futures_core::TryFuture;
use std::future::Future;

pub trait TaskExt<R>: Task<R> + Sized {
    fn or<T: Task<R, Error = Self::Error>>(self, task: T) -> Or<Self, T> {
        Or::new(self, task)
    }

    fn then<T: Task<Self::Output>>(self, task: T) -> Pipe<Self, T> {
        Pipe::new(self, task)
    }

    fn reject(self) -> Reject<Self> {
        Reject::new(self)
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
        <F::Output as TryFuture>::Error: Into<Self::Error>,
    {
        AndThen {
            filter: self,
            callback: fun,
        }
    }

    fn and_then_reject<F, V>(self, fun: F) -> AndThen<Self, F>
    where
        Self: Sized,
        Self::Output: Extract<R>,
        F: Func<<Self::Output as Extract<R>>::Extract> + Clone,
        F::Output: Future<Output = Result<V, Rejection<R, Self::Error>>>,
    {
        AndThen {
            filter: self,
            callback: fun,
        }
    }

    fn filter_pipe<F>(self, other: F) -> FilterPipe<Self, F>
    where
        Self: Sized,
        Self::Output: Extract<R>,
        F: Task<Self::Output>,
    {
        FilterPipe::new(self, other)
    }

    fn with<M>(self, middleware: M) -> M::Task
    where
        Self: Sized,
        M: Middleware<R, Self>,
    {
        middleware.wrap(self)
    }

    // fn and_with<F, U>(self, middleware: F)
    // where
    //     Self: Sized,
    //     F: Fn(R, Self) -> U,
    //     U: Future,
    // {
    // }

    fn unify<T>(self) -> Unify<Self>
    where
        Self: Task<R, Output = Either<(R, T), (R, T)>> + Sized,
        T: Tuple,
    {
        Unify { filter: self }
    }

    fn unify2<T>(self) -> Unify2<Self>
    where
        Self: Task<R, Output = Either<T, T>> + Sized,
    {
        Unify2 { filter: self }
    }

    fn map_err<F, E>(self, cb: F) -> MapErr<Self, F, E>
    where
        F: Fn(Self::Error) -> E,
    {
        MapErr::new(self, cb)
    }

    fn boxed(self) -> BoxTask<R, Self::Output, Self::Error>
    where
        Self: Clone + Sync + Send + 'static,
        Self::Future: 'static,
    {
        boxtask(self)
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
        let t = task!(|req: i32| async move { Result::<_, _, ()>::Ok(req + 1) });

        let ret = futures::executor::block_on(t.run(1));
        assert_eq!(ret, Ok(2));
    }

    #[test]
    fn test_reject() {
        let t: TaskFn<_, _, i32, ()> = task!(|req: i32| async move { reject!(req) });

        let ret = futures::executor::block_on(t.run(1));
        assert_eq!(ret, Err(Rejection::Reject(1, None)));
    }

    #[test]
    fn test_or() {
        let t = task!(|req: i32| async move {
            if req != 1 {
                reject!(req);
            } else {
                Result::<_, _, ()>::Ok(req + 1)
            }
        })
        .or(task!(|req| async move {
            if req != 2 {
                reject!(req);
            }
            Result::<_, _, ()>::Ok(req + 2)
        }))
        .or(task!(|req| async move { Ok(req + 3) }));

        // let ret = futures::executor::block_on(t.run(1));
        // assert_eq!(ret, Ok(2));

        // let ret = futures::executor::block_on(t.run(2));
        // assert_eq!(ret, Ok(4));

        // let ret = futures::executor::block_on(t.run(3));
        // assert_eq!(ret, Ok(6));

        // let ret = futures::executor::block_on(t.run(6));
        // assert_eq!(ret, Ok(9));
    }

    #[test]
    fn test_then() {
        let t = task!(|req: i32| async move { Result::<_, _, ()>::Ok(format!("{}", req + 1)) })
            .then(task!(|req: String| async move {
                let p: i32 = req.parse().unwrap();
                Result::<_, _, ()>::Ok(p + 1)
            }))
            .then(task!(
                |req: i32| async move { Result::<_, _, ()>::Ok(req + 1) }
            ));
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
            Result::<_, _, ()>::Ok((req, (name,)))
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

    // #[test]
    // fn test_map2() {
    //     let m = task!(|req: Person| async move {
    //         let name = req.name.clone();
    //         let age = req.age;
    //         Result::<_, Rejection<_, ()>>::Ok((req, (name, age)))
    //     })
    //     // .and(task!(|req: Person| async move {
    //     //     let age = req.age;
    //     //     Ok((req, (age,)))
    //     // }))
    //     .and_then(|name, age| async move { Result::<_, ()>::Ok((name, age)) })
    //     .unroll();
    //     //.//untuple_one();
    //     //.then(task!(|ret: (String, u8)| { futures::future::ok(ret) }));

    //     let ret = futures::executor::block_on(m.run(Person {
    //         name: "Rasmus".to_owned(),
    //         age: 36,
    //     }));

    //     assert!(ret.is_ok());
    //     let ret = ret.unwrap();
    //     assert_eq!((String::from("Rasmus"), 36), ret);
    // }

    // #[test]
    // fn middleware() {
    //     let t = task!(|i: i32| async move { Result::<_, Rejection<_, ()>>::Ok(i + 2) })
    //         .and_with(|req, task| async move { task.run(req) });

    //     t.run(100);
    // }
}
