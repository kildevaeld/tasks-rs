use mime;
use tasks_assets::{cache, sources, AssetRequest, Assets, Options};
use tasks_vinyl::filters;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new(cache::null(), sources::dir("."));

    tokio::spawn(async move {
        let resp = assets.get(AssetRequest::new("src")).await.unwrap();

        //let resp = future.await.unwrap();
        // let resp = assets.run("tasks", Options::default()).await.unwrap();

        println!("Node {:?}", resp.node());
    })
    .await?;

    Ok(())
}
