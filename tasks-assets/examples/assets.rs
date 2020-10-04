// #![type_length_limit = "1164189"]

use mime;
use mime::Mime;
use std::str::FromStr;
use tasks::{task, Rejection, TaskExt};
use tasks_assets::{cache, mount, sources, AssetRequest, Assets, Error, Options};
use tasks_vinyl::{filters, File};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Assets::new(
        sources::dir("tasks")
            .or(sources::dir("band"))
            .or(mount("/test", sources::dir("tasks-assets"))),
    )
    // .transform(
    //     filters::mime::match_exact(Mime::from_str("text/x-toml").unwrap()).filter_pipe(task!(
    //         |(mut file, ()): (File, ())| async move {
    //             //
    //             let name = file.path().filename().unwrap().to_owned();
    //             file.path_mut().set_filename(format!("rapper-{}", name));
    //             Ok(file)
    //         }
    //     )),
    // )
    .build();

    tokio::spawn(async move {
        let resp = assets
            .get(AssetRequest::new("test/Cargo.toml"))
            .await
            .unwrap();

        //let resp = future.await.unwrap();
        // let resp = assets.run("tasks", Options::default()).await.unwrap();

        println!("Node {:?}", resp);
    })
    .await?;

    Ok(())
}
