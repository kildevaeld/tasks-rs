#[macro_export]
macro_rules! task {
    ($task: expr) => {
        $crate::TaskFn::new($task)
    };
}

#[macro_export]
macro_rules! reject {
    ($req: expr) => {
        return Err(Rejection::Reject($req));
    };
}

#[macro_export]
macro_rules! fail {
    ($err: expr) => {
        return Err(Rejection::Err($err));
    };
}

#[macro_export]
macro_rules! middleware {
    ($m: expr) => {
        $crate::MiddlewareFn::new($m)
    };
}
