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
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use console::style;
use futures::future::join_all;
use rusoto_ecs::EcsClient;

fn get_cluster_short_name(cluster: &String) -> String {
    cluster.split("/").last().clone().unwrap().to_owned()
}

fn get_image_short_name(image: &String) -> String {
    image.split("/").last().clone().unwrap().to_owned()
}

fn print_results(
    all_clusters_images: &Vec<HashMap<String, Vec<String>>>,
    roles: &Vec<String>,
    config: &Config,
) {
    for (idx, clusters_images) in all_clusters_images.iter().enumerate() {
        let role = roles.get(idx).unwrap();
        let role_short_name = match config.roles.get(role) {
            Some(r) => r,
            None => role,
        };
        println!("{}:", style(role_short_name).cyan());
        for (cluster, images) in clusters_images {
            println!("  {}:", style(get_cluster_short_name(cluster)).green());
            let mut short_images: Vec<String> = images.iter().map(get_image_short_name).collect();
            short_images.sort();
            for image in short_images {
                println!("    {}", get_image_short_name(&image));
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let mut roles: Vec<String> = env::args().skip(1).collect();

    let mut config = Config::load(roles.is_empty())?;
    if roles.is_empty() {
        for role_arn in (&config.roles).keys() {
            roles.push(role_arn.clone());
        }
    }
    let client = Arc::new(client::new_client()?);

    credentials::update_temp_credentials(&mut config, client.clone()).await?;

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

    let images_of_clusters_res: Result<Vec<HashMap<String, Vec<String>>>> =
        get_images_of_clusters_results
            .into_iter()
            .map(|ic| ic)
            .collect();
    let cluster_images = images_of_clusters_res?;
    print_results(&cluster_images, &roles, &config);

    Ok(())
}
