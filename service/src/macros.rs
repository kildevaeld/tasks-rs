#[macro_export]
macro_rules! service {
    ($task: expr) => {
        $crate::ServiceFn::new($task)
    };
}

#[macro_export]
macro_rules! reject {
    ($req: expr) => {
        return Err($crate::Rejection::Reject($req, None));
    };
    ($req: expr, $err: expr) => {
        return Err($crate::Rejection::Reject($req, Some($err)));
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
macro_rules! or {
    [ $y: expr, $( $x:expr ),* ] => {
        {
            use $crate::ServiceExt;
            let m = $y;
            $(
                let m = m.or_else($x);
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
macro_rules! or_unify {
    [ $y: expr, $( $x:expr ),* ] => {
        {
            use $crate::{ServiceExt};
            let m = $y;
            $(
                let m = m.or_else($x).unify();
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
