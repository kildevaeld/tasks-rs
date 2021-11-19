use super::Error;
use futures_util::future::{BoxFuture, FutureExt, TryFutureExt};
use itertools::Itertools;
use service::{Rejection, Service};
use slotmap::{DefaultKey, DenseSlotMap, SecondaryMap};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::marker::PhantomData;

pub type Action<C> = Box<
    dyn Service<
        C,
        Output = (C, ()),
        Error = Error,
        Future = BoxFuture<'static, Result<(C, ()), Rejection<C, Error>>>,
    >,
>;

pub struct TaskDesc<C> {
    name: String,
    action: Action<C>,
    dependencies: Vec<String>,
}

pub struct ActionBox<A, C>(A, PhantomData<C>)
where
    A: Service<C, Output = (C, ())>;

impl<A, C> Service<C> for ActionBox<A, C>
where
    A: Service<C, Output = (C, ())>,
    A::Future: Send + 'static,
    A::Error: Into<Error>,
{
    type Future = BoxFuture<'static, Result<(C, ()), Rejection<C, Self::Error>>>;
    type Error = Error;
    type Output = (C, ());

    fn call(&self, ctx: C) -> Self::Future {
        self.0
            .call(ctx)
            .map_err(|err| match err {
                Rejection::Err(err) => Rejection::Err(err.into()),
                Rejection::Reject(ctx, Some(err)) => Rejection::Reject(ctx, Some(err.into())),
                Rejection::Reject(ctx, None) => Rejection::Reject(ctx, None),
            })
            .boxed()
    }
}

fn resolve_task<C>(
    tasks: &DenseSlotMap<DefaultKey, (String, Action<C>)>,
    map: &HashMap<DefaultKey, Vec<DefaultKey>>,
    root: &Vec<DefaultKey>,
    deps: &Vec<DefaultKey>,
) -> Result<Vec<DefaultKey>, Error> {
    let deps: Vec<_> = deps
        .iter()
        .map(|m| {
            let t = tasks.get(*m).unwrap();
            if root.contains(m) {
                return Err(Error::InvalidDepency(format!(
                    "task '{}' cannot depend on self",
                    t.0
                )));
            }
            let mut roots = root.clone();
            roots.push(*m);
            resolve_task(tasks, map, &roots, map.get(m).unwrap())
        })
        .try_collect()?;

    let mut deps = deps.into_iter().flatten().collect::<Vec<_>>();

    deps.push(*root.last().unwrap());

    let mut seen = HashSet::new();

    Ok(deps
        .into_iter()
        .filter(|m| {
            if seen.contains(m) {
                false
            } else {
                seen.insert(*m);
                true
            }
        })
        .collect())
}

pub fn sort<C>(input: Vec<TaskDesc<C>>) -> Result<Band<C>, Error> {
    let mut tasks = DenseSlotMap::default();
    let mut byname_tmp = HashMap::default();
    let mut pending = HashMap::<String, Vec<String>>::new();
    let mut tmp = HashMap::<String, DefaultKey>::new();
    let mut byname = HashMap::new();

    for task in input.into_iter() {
        let name = task.name;
        let deps = task.dependencies;
        let t = tasks.insert((name.clone(), task.action));
        tmp.insert(name.clone(), t);
        pending.insert(name, deps);
    }

    for (task, deps) in pending.into_iter() {
        let deps: Vec<_> = deps
            .into_iter()
            .map(|dep| match tmp.get(&dep) {
                Some(s) => Ok(*s),
                None => Err(Error::TaskNotFound(dep)),
            })
            .try_collect()?;

        let root = *tmp.get(&task).unwrap();
        byname_tmp.insert(root, deps);
        byname.insert(task, root);
    }

    let mut dependencies = SecondaryMap::new();
    for (root, deps) in byname_tmp.iter() {
        let t = resolve_task(&tasks, &byname_tmp, &vec![*root], &deps)?;
        dependencies.insert(*root, t);
    }

    Ok(Band {
        tasks,
        tasks_by_name: byname,
        dependencies,
    })
}

pub struct BandBuilder<C> {
    tasks: Vec<TaskDesc<C>>,
}

impl<C> BandBuilder<C> {
    pub fn add_task<A>(mut self, name: impl ToString, builder: TaskBuilder<A, C>) -> Self
    where
        A: Service<C, Output = (C, ())> + 'static,
        A::Future: Send,
        A::Error: Into<Error>,
        C: 'static,
    {
        self.tasks.push(builder.build(name.to_string()));
        self
    }

    pub fn build(self) -> Result<Band<C>, Error> {
        sort(self.tasks)
    }
}

pub struct Band<C> {
    tasks: DenseSlotMap<DefaultKey, (String, Action<C>)>,
    dependencies: SecondaryMap<DefaultKey, Vec<DefaultKey>>,
    tasks_by_name: HashMap<String, DefaultKey>,
}

impl<C> Band<C> {
    pub fn new() -> BandBuilder<C> {
        BandBuilder { tasks: Vec::new() }
    }
    pub async fn run(&self, task: &str, ctx: C) -> Result<(), Error> {
        self.run_tasks(&[task], ctx).await
    }

    pub async fn run_tasks(&self, tasks: &[&str], mut ctx: C) -> Result<(), Error> {
        let tasks = self.get_all_tasks(tasks)?;
        for task in tasks {
            let (c, _) = match self.tasks[task].1.call(ctx).await {
                Ok(c) => c,
                Err(err) => match err {
                    Rejection::Err(err) => return Err(err),
                    Rejection::Reject(_, Some(err)) => return Err(err),
                    Rejection::Reject(_, None) => return Err(Error::Rejected),
                },
            };
            ctx = c;
        }
        Ok(())
    }

    fn get_all_tasks(&self, tasks: &[&str]) -> Result<Vec<DefaultKey>, Error> {
        let mut dependencies: Vec<DefaultKey> = Vec::new();
        for task in tasks {
            let task = match self.tasks_by_name.get(*task) {
                Some(s) => s,
                None => return Err(Error::TaskNotFound((*task).to_owned())),
            };

            dependencies.extend(self.dependencies[*task].iter());
        }

        let mut seen = HashSet::new();

        Ok(dependencies
            .into_iter()
            .filter(|m| {
                if seen.contains(m) {
                    false
                } else {
                    seen.insert(*m);
                    true
                }
            })
            .collect())
    }

    pub fn get_tasks(&self, tasks: &[&str]) -> Option<Vec<&String>> {
        let dependencies = match self.get_all_tasks(tasks) {
            Ok(s) => s,
            Err(_) => return None,
        };

        let deps = dependencies
            .iter()
            .map(|m| &self.tasks.get(*m).unwrap().0)
            .collect();

        Some(deps)
    }
}

pub struct TaskBuilder<A, C>
where
    A: Service<C, Output = (C, ())>,
{
    action: A,
    dependencies: Vec<String>,
    _c: PhantomData<C>,
}

impl<A, C> TaskBuilder<A, C>
where
    A: Service<C, Output = (C, ())> + 'static,
    A::Future: Send,
    A::Error: Into<Error>,
    C: 'static,
{
    pub fn new(action: A) -> TaskBuilder<A, C> {
        TaskBuilder {
            action,
            dependencies: Vec::new(),
            _c: PhantomData,
        }
    }

    pub fn add_dependency(mut self, name: impl ToString) -> Self {
        self.dependencies.push(name.to_string());
        self
    }

    fn build(self, name: String) -> TaskDesc<C> {
        TaskDesc {
            name,
            action: Box::new(ActionBox::<A, C>(self.action, PhantomData)),
            dependencies: self.dependencies,
        }
    }
}

pub enum Dependency<T> {
    Single(T),
    Parallel(Vec<T>),
}

// impl<T> Dependency<T> {
//     fn run<C: Clone>(
//         &self,
//         tasks: &DenseSlotMap<DefaultKey, (String, Action<C>)>,
//         ctx: C,
//     ) -> Result<impl Future + Send + 'static, Error> {
//         let v = match self {
//             Dependency::Single(s) => {
//                 let action = match tasks.get(s){
//                     Some(s) => s,
//                     None => return Err(Error::Rejected)
//                 };
//                 action.1.run(ctx)
//             },
//             Dependency::Parallel(p) => {
//             }
//         }
//     }
// }

#[cfg(test)]
mod test {

    use super::*;
    use futures_util::future;
    use service::service;

    struct Test;
    impl<C: Send> Service<C> for Test {
        type Future = future::Ready<Result<(C, ()), Rejection<C, Error>>>;
        type Error = Error;
        type Output = (C, ());
        fn call(&self, ctx: C) -> Self::Future {
            future::ok((ctx, ()))
        }
    }

    #[test]
    fn test() {
        let band = Band::new();

        let band = band
            .add_task(
                "main",
                TaskBuilder::new(Test)
                    // .add_dependency("clean")
                    .add_dependency("build"),
            )
            .add_task(
                "build",
                TaskBuilder::new(service!(|_| async move {
                    Result::<_, Rejection<_, Error>>::Ok(((), ()))
                }))
                // .add_dependency("clean")
                .add_dependency("build:sass"),
            )
            .add_task("clean", TaskBuilder::new(Test).add_dependency("clean:sass"))
            .add_task(
                "build:sass",
                TaskBuilder::new(Test).add_dependency("clean:sass"),
            )
            .add_task("clean:sass", TaskBuilder::new(Test))
            .build()
            .unwrap();

        println!(
            "TASKS {:?}",
            band.get_tasks(&["clean", "build", "build:sass"])
        );

        band.run("build", ());
    }
}
