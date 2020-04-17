use serde::Deserialize;
use tasks::*;
use tasks_http::prelude::*;
use tokio;

#[derive(Deserialize, Debug)]
struct Test {
    id: Option<i32>,
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    pretty_env_logger::init();
    tasks_http::serve(
        filter_fn(|_| async move { Ok(()) })
            .and(filters::get())
            .and(filters::qs::<Test>())
            .map(|q| format!("QUERY {:?}", q))
            // .unroll()
            // .then(task!(|req: (_,)| async move { Ok(req.0) })),
    )
    .run(([127, 0, 0, 1], 3030))
    .await;

    Ok(())
}
