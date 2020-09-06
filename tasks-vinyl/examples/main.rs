use futures_util::io::AsyncReadExt;
use futures_util::pin_mut;
use futures_util::stream::{StreamExt, TryStreamExt};
use tasks::task;
use tasks::*;
use tasks_vinyl::*;
use vfs_async::{PhysicalFS, VFS};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vfs = PhysicalFS::new("./")?;
    let out = PhysicalFS::new("./output")?;
    let out = src(vfs, "**/*.rs")
        .await?
        // .pipe(filters::mime_exact(mime::Mime).pipe(task!(|file| async move { Ok(file) })))
        .pipe(task!(|file| async move { Ok(file) }))
        .pipe(out.path("."))
        .buffered(10)
        .collect::<Vec<_>>()
        .await;

    println!("OUT {:?}", out);

    Ok(())
}
