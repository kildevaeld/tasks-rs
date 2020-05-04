use super::{Middleware, Task};

#[derive(Clone)]
pub struct Stack<M1, M2> {
    m1: M1,
    m2: M2,
}

impl<M1, M2> Stack<M1, M2> {
    pub fn new(m1: M1, m2: M2) -> Stack<M1, M2> {
        Stack { m1, m2 }
    }
}

impl<M1, M2, R, T> Middleware<R, T> for Stack<M1, M2>
where
    M1: Send + Middleware<R, M2::Task>,
    M2: 'static + Clone + Send + Sync + Middleware<R, T>,
    T: Task<R>,
    R: 'static,
{
    type Task = M1::Task;
    fn wrap(&self, task: T) -> Self::Task {
        self.m1.wrap(self.m2.wrap(task))
    }
}
