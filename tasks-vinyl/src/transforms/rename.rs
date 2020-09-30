use crate::{filters, Error, File};
use pathutils;
use tasks::{task, Task};

pub fn set_ext(ext: impl ToString) -> impl Task<File, Output = File, Error = Error> + Clone {
    let ext = ext.to_string();
    task!(move |mut file: File| {
        file.path_mut().set_ext(&ext);
        futures_util::future::ok(file)
    })
}
