mod and;
mod and_then;
mod any;
mod filter;
mod filter_ext;
mod generic;
mod map;

pub use self::{and::*, and_then::*, any::*, filter::*, filter_ext::*, generic::*, map::*};

#[cfg(test)]
mod test {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn test_filter() {
        let f = filter_fn_one(|req: &mut String| async { Result::<_, ()>::Ok("Hello, World") })
            .and(filter_fn_one(|req: &mut String| async {
                Result::<_, ()>::Ok("Hello, World2")
            }))
            .map(|req1, req2| format!("{} {}", req1, req2))
            .and_then(|req| async move { Result::<_, ()>::Ok(format!("{} x 2", req)) });

        let name = String::from("Hello");
        let out = block_on(f.filter(name));
        assert!(out.is_ok());
        let (_, out) = out.unwrap();
        assert_eq!(("Hello, World Hello, World2 x 2".to_owned(),), out);
    }

    #[test]
    fn test_any() {
        let filter = any().map(|| "Hello, World");

        let out = block_on(filter.filter(100));
    }
}
