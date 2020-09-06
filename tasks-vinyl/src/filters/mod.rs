use crate::{Error, File};
use mime::Mime;
use tasks::{task, Rejection, Task, TaskExt};

pub fn mime_exact(mime: Mime) -> impl Task<File, Output = (File, ()), Error = Error> + Clone {
    task!(move |file: File| {
        let mime = mime.clone();
        async move {
            if file.mime == mime {
                Ok((file, ()))
            } else {
                Err(Rejection::Reject(file, None))
            }
        }
    })
}

pub fn mime_type(
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

pub fn any() -> impl Task<File, Output = (File, ()), Error = Error> + Copy {
    task!(|file| async move { Ok((file, ())) })
}

pub fn state<S: Send + Clone + 'static>(
    state: S,
) -> impl Task<File, Output = (File, (S,)), Error = Error> + Clone {
    any().map(move || state.clone())
}
