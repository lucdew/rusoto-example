#[macro_use]
extern crate log;

#[macro_use]
extern crate anyhow;

mod client;
mod config;
mod credentials;
mod ecs;

use anyhow::Result;
use config::Config;
use std::env;
use std::sync::Arc;

use futures::future::join_all;
use rusoto_ecs::EcsClient;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut config = Config::load()?;
    let client = Arc::new(client::new_client()?);

    credentials::update_temp_credentials(&mut config, client.clone()).await?;

    let roles: Vec<String> = env::args().skip(1).collect();

    let get_creds_futures = roles
        .iter()
        .map(|role_arn| credentials::assume_role(&config, client.clone(), role_arn));

    let get_creds = join_all(get_creds_futures).await;

    let all_creds_res: Result<Vec<credentials::Credentials>> =
        get_creds.into_iter().map(|c| c).collect();

    let ecs_clients: Vec<EcsClient> = all_creds_res?
        .into_iter()
        .map(|creds| ecs::build_ecs_client(client.clone(), creds))
        .collect();

    let get_images_of_clusters_results = join_all(
        ecs_clients
            .iter()
            .map(|ecs_client| ecs::get_images_of_clusters(&ecs_client)),
    )
    .await;

    for images_of_cluster in get_images_of_clusters_results {
        println!("{:#?}", images_of_cluster?);
    }

    Ok(())
}
