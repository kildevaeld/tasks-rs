// #![type_length_limit = "1164189"]

use mime;
use mime::Mime;
use std::str::FromStr;
use tasks::{task, Rejection, TaskExt};
use tasks_assets::{cache, mount, sources, AssetRequest, Assets, Error, Options};
use tasks_vinyl::{filters, File};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new(tasks_assets::mount("/test", sources::dir(".."))).build();

    let resp = assets.get(AssetRequest::new("/test")).await.unwrap();

    //let resp = future.await.unwrap();
    // let resp = assets.run("tasks", Options::default()).await.unwrap();

    println!("Node {:?}", resp);

    Ok(())
}
