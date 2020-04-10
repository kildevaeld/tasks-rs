use tasks_http::prelude::*;
use tokio;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(tasks_http::ws(|ws: tasks_http::Ws| async move {
        Result::<_, Error>::Ok(ws.on_upgrade(|ws| async move {}))
    }))
    //
    .run(([127, 0, 0, 1], 3030))
    .await;

    Ok(())
}
