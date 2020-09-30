use crate::{Error, File};
use mime::Mime;
use tasks::{filter_fn_one, task, Rejection, Task, TaskExt};

pub fn match_exact(mime: Mime) -> impl Task<File, Output = (File, ()), Error = Error> + Clone {
    task!(move |file: File| {
        let mime = mime.clone();
        async move {
            println!("file {} {}", file.mime, mime);
            if file.mime == mime {
                Ok((file, ()))
            } else {
                let mime = file.mime.clone();
                Err(Rejection::Reject(
                    file,
                    Some(Error::InvalidMimeType { expected: mime }),
                ))
            }
        }
    })
}

pub fn match_type(
    mime: impl ToString,
) -> impl Task<File, Output = (File, ()), Error = Error> + Clone {
    let mime = mime.to_string();
    task!(move |file: File| {
        let mime = mime.clone();
        async move {
            if file.mime.type_().as_str() == &mime {
                Ok((file, ()))
            } else {
                Err(Rejection::Reject(file, None))
            }
        }
    })
}

pub fn get() -> impl Task<File, Output = (File, (Mime,)), Error = Error> + Copy {
    task!(|file: File| {
        let mime = file.mime.clone();
        futures_util::future::ok((file, (mime,)))
    })
}
