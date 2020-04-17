use tasks_http::prelude::*;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(
        tasks_http::mount(
            "/test",
            task!(|_| async move {
                println!("HERE /");
                Ok(Response::with("This").set(StatusCode::OK))
            }),
        )
        .or(tasks_http::filters::mount("/mount").map(|| "Hello, mount")),
    )
    .run(([127, 0, 0, 1], 3030))
    .await;

    Ok(())
}
