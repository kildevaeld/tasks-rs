mod and;
mod filter;
mod filter_ext;
mod generic;
mod map;

pub use self::{and::*, filter::*, filter_ext::*, generic::*, map::*};

#[cfg(test)]
mod test {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn test_filter() {
        let f = filter_fn_one(|req: &String| async { Result::<_, ()>::Ok("Hello, World") })
            .and(filter_fn_one(|req: &String| async {
                Result::<_, ()>::Ok("Hello, World2")
            }))
            .map(|req1, req2| format!("{} {}", req1, req2));

        let name = String::from("Hello");
        let out = block_on(f.filter(&name));
        assert!(out.is_ok());
        let out = out.unwrap();
        assert_eq!(("Hello, World Hello, World2".to_owned(),), out);
    }
}
