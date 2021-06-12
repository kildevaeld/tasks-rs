use super::{
    and::And, and_then::AndThen, and_then_reject::AndThenReject, err_into::ErrInto, map::Map,
    map_err::MapErr, or::Or, unify::Unify, unpack::Unpack, Combine, Either, Extract, Func, Service,
    Tuple,
};
use futures_core::TryFuture;

pub trait ServiceExt<R>: Service<R> + Sized {
    fn or<T: Service<R>>(self, task: T) -> Or<Self, T> {
        Or::new(self, task)
    }

    fn unify<T, E>(self) -> Unify<Self>
    where
        Self: Service<R, Output = Either<T, T>, Error = Either<E, E>> + Sized,
    {
        Unify { filter: self }
    }

    fn map_err<F, E>(self, cb: F) -> MapErr<Self, F, E>
    where
        F: Fn(Self::Error) -> E,
    {
        MapErr::new(self, cb)
    }

    fn err_into<E>(self) -> ErrInto<Self, E>
    where
        E: From<Self::Error>,
    {
        ErrInto::new(self)
    }
}

pub trait ServiceExtract<R>: Service<R> + Sized {
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

    fn and_then_reject<F>(self, other: F) -> AndThenReject<Self, F>
    where
        Self: Sized,
        Self::Output: Extract<R>,
        F: Service<Self::Output>,
    {
        AndThenReject::new(self, other)
    }

    fn map<F>(self, fun: F) -> Map<Self, F>
    where
        Self::Output: Extract<R>,
        F: Func<<Self::Output as Extract<R>>::Extract> + Clone,
    {
        Map {
            filter: self,
            callback: fun,
        }
    }

    fn and<F>(self, other: F) -> And<Self, F>
    where
        Self: Sized,
        Self::Output: Extract<R>,
        <<Self::Output as Extract<R>>::Extract as Tuple>::HList:
            Combine<<<F::Output as Extract<R>>::Extract as Tuple>::HList>,
        F: Service<R> + Clone,
        F::Output: Extract<R>,
    {
        And {
            first: self,
            second: other,
        }
    }

    fn unpack(self) -> Unpack<Self>
    where
        Self: Sized,
        Self::Output: Extract<R>,
    {
        Unpack(self)
    }
}

impl<R, T> ServiceExt<R> for T where T: Service<R> {}

impl<R, T> ServiceExtract<R> for T
where
    T: Service<R>,
    T::Output: Extract<R>,
{
}
