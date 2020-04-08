use tasks_core::{reject, task, Rejection, TaskExt};
use tasks_http::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(tasks_http::compress().end(task!(|req: Request| async move {
        Ok(Response::with(StatusCode::OK).set("Hello, World!"))
    })))
    .run(([127, 0, 0, 1], 3030))
    .await;

    Ok(())
}
