mod and;
mod and_then;
mod map;
mod unroll;
mod untuple;

pub use self::{and::*, and_then::*, map::*, unroll::*, untuple::*};

use crate::{Combine, Func, Rejection, Task, Tuple};

pub trait Extract<R>: Sized {
    type Extract: Tuple + Send;
    fn unpack(self) -> (R, Self::Extract);
}

impl<R, U> Extract<R> for (R, U)
where
    U: Tuple + Send,
{
    type Extract = U;
    fn unpack(self) -> (R, Self::Extract) {
        self
    }
}

use futures_core::TryFuture;

pub trait FilterExt<R>: Task<R> {
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

    fn untuple_one(self) -> Untuple<Self>
    where
        Self: Sized,
        Self::Output: Tuple,
    {
        Untuple(self)
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

impl<'a, R, T> FilterExt<R> for T
where
    T: Task<R>,
    T::Output: Extract<R>,
{
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::*;

    #[derive(Debug)]
    struct Person {
        name: String,
        age: u8,
    }
    #[test]
    fn test_map() {
        let m = task!(|req: Person| async move {
            let name = req.name.clone();
            Result::<_, Rejection<_, ()>>::Ok((req, (name,)))
        })
        .and(task!(|req: Person| async move {
            let age = req.age;
            Ok((req, (age,)))
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
