// use tasks::{reject, task, Rejection, Task, TaskExt};
use tasks_http::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(task!(|req: Request| async move {
        Ok(Response::with("Hello, World").set(StatusCode::OK))
    }))
    .tls()
    .cert_path("examples/tls/cert.pem")
    .key_path("examples/tls/key.rsa")
    .run(([127, 0, 0, 1], 3030))
    .await;

    Ok(())
}
