use tasks::*;
use tasks_vinyl::*;
use futures_util::io::AsyncReadExt;
use futures_util::pin_mut;

#[async_std::main]
async fn main() -> Result<(), Error> {
    let dir = DirectoryProducer::new("./tasks/src").await?;

    dir.into_vinyl().with_task(task_fn!(|mut item: File| {
        async move {
            //let mut output = Vec::default();
            let out = match item.content {
                Content::Bytes(bytes) => {
                    bytes
                },
                _ => {
                    return Ok(())
                }
            };
            println!("PATH {:?} {:?}", item.path, out.len());
            Ok(())
        }
    })).consume(5).await?;

    Ok(())
}
