use futures_util::io::AsyncReadExt;
use futures_util::pin_mut;
use futures_util::stream::{StreamExt, TryStreamExt};
use tasks::task;
use tasks::*;
use tasks_vinyl::*;
use vfs_async::{PhysicalFS, VFS};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vfs = PhysicalFS::new("../../Bolighed/mithjem")?;
    tasks_vinyl::runtime::mkdir("./output").await;
    let out = PhysicalFS::new("./output")?;

    let streams = Builder::new();

    // let out = streams
    //     .push(
    //         src(vfs, "**/*.*")
    //             .await?
    //             .pipe(task!(|file: File| async move {
    //                 println!("file {}", file.path);
    //                 Ok(file)
    //             }))
    //             .pipe(PathTask::new(out.path("plain")).overwrite(true)),
    //     )
    //     .run()
    //     .await;

    let out = src(vfs, "**/*.*")
        .await?
        // .pipe(filters::mime_exact(mime::Mime).pipe(task!(|file| async move { Ok(file) })))
        //.pipe(task!(|file| async move { Ok(file) }))
        .pipe(PathTask::new(out.path("plain")).overwrite(true))
        //.pipe(transforms::set_ext("rapper"))
        // .pipe(task!(|file: File| async move {
        //     println!("file {}", file.path);
        //     Ok(file)
        // }))
        //.write_to(out.path("other").to_dest())
        //.map(|m| async move { m.await })
        //.buffered(20)
        //.then(|m| async move { m.await })
        // .try_collect::<Vec<_>>()
        // .for_each(|file| async move {
        //     //println!("file {:?}", file.unwrap().path);
        // })
        .write_to(Discard)
        .await?;

    println!("OUT {:?}", out);

    Ok(())
}
