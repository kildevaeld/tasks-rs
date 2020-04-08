use tasks_core::{reject, task, Rejection, Task, TaskExt};
use tasks_http::{BoxError, Error, Request, Response};
use tokio;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(task!(|req: Request| async move {
        println!("HERE /");
        Ok(Response::new())
    }))
    .tls()
    .cert_path("examples/tls/cert.pem")
    .key_path("examples/tls/key.rsa")
    .run(([127, 0, 0, 1], 3030))
    .await;

    Ok(())
}
