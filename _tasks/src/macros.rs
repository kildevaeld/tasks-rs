


#[macro_export]
macro_rules! task_fn {
    ($handler: expr) => {
        $crate::TaskFn::new($handler, |_| true)
    };
    ($handler: expr, $check: expr) => {
        $crate::TaskFn::new($handler, $check)
    };
}


#[macro_export]
macro_rules! middleware_fn {
    ($handler: expr) => {
        $crate::MiddlewareFn::new($handler)
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
macro_rules! either {
    [ $y: expr, $( $x:expr ),* ] => {
        {
            use $crate::TaskExt;
            let m = $y;
            $(
                let m = m.or($x);
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

#[cfg(feature = "sync")]
#[macro_export]
macro_rules! pool {
    ($size: expr, $func: expr) => {
        $crate::sync::Pool::new($size, $func)
    };
}

#[cfg(feature = "sync")]
#[macro_export]
macro_rules! sync_task_fn {
    ($handler: expr) => {
        $crate::sync::SyncTaskFn::new($handler, |_| true)
    };
    ($handler: expr, $check: expr) => {
        $crate::sync::SyncTaskFn::with_check($handler, $check)
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

    #[cfg(feature = "sync")]
    #[test]
    fn test_pool() {
        let pool = pool!(2, sync_task_fn!(|test| Result::<_, TaskError>::Ok(test + 2))).unwrap();

        let ans = futures_executor::block_on(pool.exec(2));
    }

}