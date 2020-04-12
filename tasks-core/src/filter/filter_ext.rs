use super::{And, AndThen, Combine, Filter, Func, Map, Tuple};
use futures_core::TryFuture;

pub trait FilterExt<R>: Filter<R> {
    fn and<F>(self, other: F) -> And<Self, F>
    where
        Self: Sized,
        <Self::Extract as Tuple>::HList: Combine<<F::Extract as Tuple>::HList>,
        F: Filter<R> + Clone,
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
        F: Func<Self::Extract> + Clone,
    {
        Map {
            filter: self,
            callback: fun,
        }
    }

    fn and_then<F>(self, fun: F) -> AndThen<Self, F>
    where
        Self: Sized,
        F: Func<Self::Extract> + Clone,
        F::Output: TryFuture + Send,
        <F::Output as TryFuture>::Error: Into<Self::Error>, //CombineRejection<Self::Error>,
    {
        AndThen {
            filter: self,
            callback: fun,
        }
    }
}

impl<'a, R, T> FilterExt<R> for T where T: Filter<R> {}
