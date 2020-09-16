use super::Error;
use futures_util::future::{BoxFuture, FutureExt};
use itertools::Itertools;
use slotmap::{DefaultKey, DenseSlotMap, SecondaryMap};
use std::collections::{HashMap, HashSet};
use std::future::Future;

pub trait Action {
    type Future: Future<Output = Result<(), Error>>;
    fn call(&self) -> Self::Future;
}

pub struct TaskDesc {
    name: String,
    action: Box<dyn Action<Future = BoxFuture<'static, Result<(), Error>>>>,
    dependencies: Vec<String>,
}

pub struct ActionBox<A>(A);

impl<A> Action for ActionBox<A>
where
    A: Action,
    A::Future: Send + 'static,
{
    type Future = BoxFuture<'static, Result<(), Error>>;
    fn call(&self) -> Self::Future {
        self.0.call().boxed()
    }
}

fn resolve_task(
    tasks: &DenseSlotMap<
        DefaultKey,
        (
            String,
            Box<dyn Action<Future = BoxFuture<'static, Result<(), Error>>>>,
        ),
    >,
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

pub fn sort(input: Vec<TaskDesc>) -> Result<Band, Error> {
    let mut tasks = DenseSlotMap::default();
    let mut byname_tmp = HashMap::default();
    let mut pending = HashMap::<String, Vec<String>>::new();
    let mut tmp = HashMap::<String, DefaultKey>::new();
    let mut byname = HashMap::new();

    for task in input.into_iter() {
        let name = task.name; //.clone();
        let deps = task.dependencies; //.clone(); //.iter().map(|_| None).collect();
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

pub struct BandBuilder {
    tasks: Vec<TaskDesc>,
}

impl BandBuilder {
    pub fn add_task<A>(mut self, name: impl ToString, builder: TaskBuilder<A>) -> Self
    where
        A: Action + 'static,
        A::Future: Send,
    {
        self.tasks.push(builder.build(name.to_string()));
        self
    }

    pub fn build(self) -> Result<Band, Error> {
        sort(self.tasks)
    }
}

pub struct Band {
    tasks: DenseSlotMap<
        DefaultKey,
        (
            String,
            Box<dyn Action<Future = BoxFuture<'static, Result<(), Error>>>>,
        ),
    >,
    dependencies: SecondaryMap<DefaultKey, Vec<DefaultKey>>,
    tasks_by_name: HashMap<String, DefaultKey>,
}

impl Band {
    pub fn new() -> BandBuilder {
        BandBuilder { tasks: Vec::new() }
    }
    pub async fn run(&self, task: &str) -> Result<(), Error> {
        let task = match self.tasks_by_name.get(task) {
            Some(s) => s,
            None => return Err(Error::TaskNotFound(task.to_owned())),
        };

        for dep in self.dependencies.get(*task).unwrap() {
            self.tasks[*dep].1.call().await?;
        }

        Ok(())
    }

    pub fn get_tasks(&self, task: &str) -> Option<Vec<&String>> {
        let task = match self.tasks_by_name.get(task) {
            Some(s) => s,
            None => return None,
        };

        let deps = self.dependencies[*task]
            .iter()
            .map(|m| &self.tasks.get(*m).unwrap().0)
            .collect();

        Some(deps)
    }
}

pub struct TaskBuilder<A> {
    action: A,
    dependencies: Vec<String>,
}

impl<A> TaskBuilder<A>
where
    A: Action + 'static,
    A::Future: Send,
{
    pub fn new(action: A) -> TaskBuilder<A> {
        TaskBuilder {
            action,
            dependencies: Vec::new(),
        }
    }

    pub fn add_dependency(mut self, name: impl ToString) -> Self {
        self.dependencies.push(name.to_string());
        self
    }

    fn build(self, name: String) -> TaskDesc {
        TaskDesc {
            name,
            action: Box::new(ActionBox(self.action)),
            dependencies: self.dependencies,
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use futures_util::future;
    struct Test;
    impl Action for Test {
        type Future = future::Ready<Result<(), Error>>;
        fn call(&self) -> Self::Future {
            future::ok(())
        }
    }

    #[test]
    fn test() {
        let mut band = Band::new();

        let band = band
            .add_task(
                "main",
                TaskBuilder::new(Test)
                    // .add_dependency("clean")
                    .add_dependency("build"),
            )
            .add_task(
                "build",
                TaskBuilder::new(Test)
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

        println!("TASKS {:?}", band.get_tasks("build"));
    }
}
