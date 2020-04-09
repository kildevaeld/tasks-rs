use tasks_http::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    let service = middleware!(|req, next| async move {
        let ret = next.run(req).await?;
        Ok(ret)
    })
    .end(task!(|_| async move { Ok("Hello, World") }));

    tasks_http::serve(service).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}
