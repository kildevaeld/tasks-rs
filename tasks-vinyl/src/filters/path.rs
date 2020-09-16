use crate::{Error, File};
use tasks::{reject, task, Rejection, Task};

pub fn get() -> impl Task<File, Output = (File, (String,)), Error = Error> + Copy {
    task!(|file: File| {
        let path = file.path.clone();
        futures_util::future::ok((file, (path,)))
    })
}

pub fn match_exact(
    path: impl ToString,
) -> impl Task<File, Output = (File, ()), Error = Error> + Clone {
    let path = path.to_string();
    task!(move |file: File| {
        let path = path.clone();
        async move {
            if file.path == path {
                Ok((file, ()))
            } else {
                // let path = file.path.clone();
                Err(Rejection::Reject(file, None))
            }
        }
    })
}

pub fn match_ext(
    ext: impl ToString,
) -> impl Task<File, Output = (File, ()), Error = Error> + Clone {
    let mut ext = ext.to_string();
    if !ext.starts_with(".") {
        ext = String::from(".") + &ext;
    }
    task!(move |file: File| {
        let ext = ext.clone();
        async move {
            let real_ext = match pathutils::extname(&file.path) {
                Some(ext) => ext,
                None => reject!(file),
            };
            if real_ext != ext {
                reject!(file);
            }
            Ok((file, ()))
        }
    })
}
