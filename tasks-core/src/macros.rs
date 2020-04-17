#[macro_export]
macro_rules! task {
    ($task: expr) => {
        $crate::TaskFn::new($task)
    };
}

#[macro_export]
macro_rules! reject {
    ($req: expr) => {
        return Err($crate::Rejection::Reject($req));
    };
}

#[macro_export]
macro_rules! fail {
    ($err: expr) => {
        return Err($crate::Rejection::Err($err));
    };
}

#[macro_export]
macro_rules! middleware {
    ($m: expr) => {
        $crate::MiddlewareFn::new($m)
    };
}

#[macro_export]
macro_rules! and {
    [ $y: expr, $( $x:expr ),* ] => {
        {
            use $crate::MiddlewareExt;
            let m = $y;
            $(
                let m = m.and($x);
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
macro_rules! or {
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
