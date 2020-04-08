use tasks_core::{reject, task, Rejection, TaskExt};
use tasks_http::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(
        task!(|req: Request| async move {
            // if req.url().path() != "/" {
            //     reject!(req);
            // }
            //Ok(Response::new())
            Ok(Response::with(StatusCode::OK).set("Hello, World!"))
        }), //.or(task!(|_| async move { Ok(Response::new()) })),
    )
    .run(([127, 0, 0, 1], 3030))
    .await;

    Ok(())
}
