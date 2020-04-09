// use tasks_core::{reject, task, Rejection, Task, TaskExt};
use tasks_http::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(task!(|req: Request| async move {
        Ok(Response::with("Hello, World").set(StatusCode::OK))
    }))
    .uds()
    .run("./test.sock")
    .await;

    Ok(())
}
