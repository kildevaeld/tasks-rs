


#[macro_export]
macro_rules! task_fn {
    ($handler: expr) => {
        $crate::task::TaskFn::new($handler)
    };
    ($handler: expr, $check: expr) => {
        $crate::task::TaskFn::with_check($handler, $check)
    };
}


#[macro_export]
macro_rules! middleware_fn {
    ($handler: expr) => {
        $crate::middleware::MiddlewareFn::new($handler)
    };
}


#[macro_export]
macro_rules! pipe {
    [ $y: expr, $( $x:expr ),* ] => {
        {
            use $crate::TaskExt;
            let m = $y;
            $(
                let m = m.pipe($x);
            )*
            m
        }
     };

     [ $y: expr] => {
        {
            $y
        }
     };
}

#[macro_export]
macro_rules! chain {
    [ $y: expr, $( $x:expr ),* ] => {
        {
            use $crate::ConditionalTaskExt;
            let m = $y;
            $(
                let m = m.chain($x);
            )*
            m
        }
     };

     [ $y: expr] => {
        {
            $y
        }
     };
}


#[macro_export]
macro_rules! stack {
    [ $y: expr, $( $x:expr ),* ] => {
        {
            use $crate::MiddlewareExt;
            let m = $y;
            $(
                let m = m.stack($x);
            )*
            m
        }
     };

     [ $y: expr] => {
        {
            $y
        }
     };
}


#[cfg(test)]
mod tests {

    // use super::super::station_fn;
    // use super::super::Station;
    use super::super::*;
    use futures_util::future;
    #[test]
    fn test_pipe() {
        let chain = pipe![
            task_fn!(|s: &str| future::ok(s) ),
            task_fn!(|s: &str| future::ok(s)),
            pipe![task_fn!(|s: &str| future::ok(s))],
            pipe![task_fn!(|s: &str| future::ok::<_, TaskError>(s))]
        ];

        let result = futures_executor::block_on(chain.exec("Hello, World!")).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_conveyor_meaning_of_life() {
        let chain = pipe![
            task_fn!(|input: &str| future::ok(input.len())),
            task_fn!(|len: usize| future::ok::<_, TaskError>(len * 7))
        ];

        let ans = futures_executor::block_on(chain.exec("Hello!"));

        assert_eq!(ans.unwrap(), 42);
    }

}