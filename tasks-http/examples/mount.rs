use tasks_http::prelude::*;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(tasks_http::mount(
        "/test",
        task!(|_| async move {
            println!("HERE /");
            Ok(Response::new())
        }),
    ))
    .run(([127, 0, 0, 1], 3030))
    .await;

    Ok(())
}
