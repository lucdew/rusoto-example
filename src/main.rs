extern crate futures;
extern crate hyper;
extern crate hyper_tls;
//extern crate rusoto_sts;
extern crate hyper_proxy;
extern crate regex;
extern crate rusoto_core;
extern crate rusoto_credential;
extern crate rusoto_ecs;
extern crate rusoto_sts;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate error_chain;
extern crate chrono;
extern crate read_input;
extern crate tokio_core;

#[macro_use]
extern crate log;
extern crate env_logger;

mod client;
mod config;
mod credentials;
mod ecs;
mod errors;

use config::Config;
use errors::*;
use futures::future::{join_all, ok, result, Future};
use std::env;
use tokio_core::reactor::Core;

use rusoto_ecs::EcsClient;

fn main() -> Result<()> {
    env_logger::init();

    let mut config = Config::load()?;
    let connector = client::new_connector()?;

    credentials::update_temp_credentials(&mut config, connector.clone())?;

    let roles: Vec<String> = env::args().skip(1).collect();

    let mut core = Core::new().unwrap();

    let get_creds = roles.iter().map(|role_arn| {
        result(credentials::assume_role(
            &config,
            connector.clone(),
            role_arn,
        ))
    });

    let all_creds = core.run(join_all(get_creds))?;

    let ecs_clients: Vec<EcsClient> = all_creds
        .into_iter()
        .map(|creds| ecs::build_ecs_client(connector.clone(), creds))
        .collect();

    let get_images_of_clusters = ecs_clients
        .iter()
        .map(|ecs_client| ecs::get_images_of_clusters(&ecs_client));

    core.run(join_all(get_images_of_clusters).and_then(|clusters_maps| {
        for clusters_map in clusters_maps {
            println!("{:#?}", clusters_map);
        }
        ok(true)
    }))?;

    Ok(())
}
