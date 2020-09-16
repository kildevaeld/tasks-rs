pub mod mime;
pub mod path;

use crate::{Error, File};
use tasks::{task, Task, TaskExt};

pub fn any() -> impl Task<File, Output = (File, ()), Error = Error> + Copy {
    task!(|file| async move { Ok((file, ())) })
}

pub fn state<S: Send + Clone + 'static>(
    state: S,
) -> impl Task<File, Output = (File, (S,)), Error = Error> + Clone {
    any().map(move || state.clone())
}
