mod error;
mod source;
mod target;

pub use self::{error::*, source::*, target::*};

#[cfg(test)]
mod test {
    use super::*;
    use tasks_vinyl::{VPathExt, VinylStream, VinylStreamDestination};
    use vfs_async::{PhysicalFS, VPath, VFS};
    #[tokio::test]
    async fn test() {
        let source = VfsSource::new(PhysicalFS::new(".").unwrap());
        let target = PhysicalFS::new("TEST_OUTPUT").unwrap();
        target.path(".").mkdir().await;
        source
            .vinyl()
            .await
            .unwrap()
            .write_to(target.path(".").to_dest())
            .await;
    }
}
