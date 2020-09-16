use crate::Action;
use futures_util::future::TryFuture;
use std::future::Future;

impl<F, R, E> Action for F
where
    F: Fn() -> R,
    R: Future<Output = Result<(), E>>,
{
    type Future = R;
    type Error = <R as TryFuture>::Error;
    fn call(&self) -> Self::Future {
        self()
    }
}

#[cfg(test)]
mod test {

    use crate::*;

    #[test]
    fn test() {
        Band::new().add_task(
            "main",
            TaskBuilder::new(|| async move {
                //
                Result::<_, Error>::Ok(())
            })
            .add_dependency(""),
        );
    }
}
