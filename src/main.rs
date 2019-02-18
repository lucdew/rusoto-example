extern crate hyper;
extern crate hyper_tls;
//extern crate rusoto_sts;
extern crate hyper_proxy;
extern crate regex;
extern crate rusoto_core;
extern crate rusoto_credential;
extern crate rusoto_sts;
extern crate rusoto_ecs;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate error_chain;
extern crate read_input;
extern crate chrono;

mod errors;
mod client;
mod config;
mod credentials;
mod ecs;


use config::Config;
use errors::*;
use std::env;

fn main() -> Result<()> {
    let mut config = Config::load()?;
    let connector = client::new_connector()?;

    credentials::update_temp_credentials(&mut config, connector.clone())?;

    let role_arn = env::args().skip(1).next().ok_or("Missing argument")?;

    let creds = credentials::assume_role(&config, connector.clone(), &role_arn)?;
    println!("Got credentials {:?}",creds);
    ecs::get_clusters(connector.clone(), creds)?;

    Ok(())
}
