use super::util;
use super::{Content, Error, File};
use futures_core::{future::BoxFuture, ready, Stream};
use futures_util::{io::AsyncReadExt, stream::Buffered, StreamExt, TryStreamExt};
use pin_project::{pin_project, project};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tasks::{task, Rejection, Task};
use vfs_async::{Globber, OpenOptions, VFile, VPath, VFS};

pub fn dest<V>(vfs: V) -> impl Task<File, Output = (), Error = Error> + Send + Clone
where
    V: VFS + Clone + 'static,
    V::Path: 'static + std::marker::Unpin + std::fmt::Debug,
    <V::Path as VPath>::ReadDir: Send,
    <V::Path as VPath>::Metadata: Send,
{
    task!(move |file: File| {
        let vfs = vfs.clone();
        async move {
            let path = vfs.path(&file.path);
            if let Some(parent) = path.parent() {
                if !parent.exists().await {
                    parent.mkdir().await.unwrap();
                    //println!("no exists");
                }
            }
            if path.exists().await {
                println!("already exists");
                return Ok(());
            } else {
                let file = path
                    .open(OpenOptions::new().create(true).write(true))
                    .await
                    .unwrap();
            }
            // println!("file {:?} {:?}", path, file);
            Ok(())
        }
    })
}
