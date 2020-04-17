use tasks_http::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();

    let h = tasks_http::mount("/test", tasks_http::handlers::dir("./"))
        .or(tasks_http::filters::any().map(|| "Not found!"));

    tasks_http::serve(h).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}
