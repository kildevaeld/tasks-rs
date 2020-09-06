mod error;
mod source;
mod target;

pub use self::{error::*, source::*, target::*};

#[cfg(test)]
mod test {
    use super::*;
    use futures_util::{stream::iter, TryFutureExt};
    use tasks::task;
    use tasks_vinyl::{
        filters, Builder, Error as VinylError, File, VPathExt, VinylStream, VinylStreamDestination,
    };
    use vfs_async::{MemoryFS, PhysicalFS, VPath, VFS};
    #[tokio::test]
    async fn test() -> Result<(), Box<dyn std::error::Error>> {
        let source = VfsSource::new(PhysicalFS::new(".").unwrap());
        let target = PhysicalFS::new("TEST_OUTPUT").unwrap();
        let res = Builder::new()
            .push(source.resource("Cargo.toml").await?.vinyl().pipe(task!(
                |mut file: File| async move {
                    file.path = String::from("/Hello_World");
                    Ok(file)
                }
            )))
            .push(source.vinyl().await?.pipe(filters::any()))
            .into_stream()
            .write_to(target.path(".").to_dest())
            .await?;

        Ok(())
    }
}
