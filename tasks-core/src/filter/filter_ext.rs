use super::{And, Combine, Filter, Func, Map, Tuple};

pub trait FilterExt<'a, R>: Filter<'a, R> {
    fn and<F>(self, other: F) -> And<Self, F>
    where
        Self: Sized,
        <Self::Extract as Tuple>::HList: Combine<<F::Extract as Tuple>::HList>,
        F: Filter<'a, R> + Clone,
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
}

impl<'a, R, T> FilterExt<'a, R> for T where T: Filter<'a, R> {}
