use tasks_http::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(task!(|_| async move { Ok("Hello, World") }))
        .run(([127, 0, 0, 1], 3030))
        .await;

    Ok(())
}
