use mime;
use tasks_assets::{cache, sources, Assets, Options};
use tasks_vinyl::filters;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new(cache::null(), sources::dir("."));

    let resp = assets.run("tasks", Options::default()).await?;

    println!("Node {:?}", resp.node());

    Ok(())
}
