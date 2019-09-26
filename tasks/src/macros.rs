


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