use crate::client::HttpClient;
use crate::config;

use anyhow::Context;
use anyhow::Result;
use dialoguer::Input;
use rusoto_core::region::Region;
use rusoto_credential::StaticProvider;
use rusoto_sts::{AssumeRoleRequest, GetSessionTokenRequest, Sts, StsClient};
use std::sync::Arc;

use chrono::prelude::*;

#[derive(Debug)]
pub struct Credentials {
    pub aws_access_key: String,
    pub aws_secret_key: String,
    pub aws_sts_token: String,
}

pub async fn update_temp_credentials(
    config: &mut config::Config,
    client: Arc<HttpClient>,
) -> Result<()> {
    if !config.is_token_valid() {
        let mfa: String = Input::<String>::new()
            .with_prompt("Please enter your mfa")
            .interact()?;

        let cred_provider = StaticProvider::new(
            config.aws_access_key_id.clone(),
            config.aws_secret_access_key.clone(),
            None,
            None,
        );

        let sts_client = StsClient::new_with(client, cred_provider, Region::EuWest1);

        let get_session_token = GetSessionTokenRequest {
            duration_seconds: None,
            serial_number: Some(
                config
                    .aws_mfa_device_arn
                    .clone()
                    .ok_or(anyhow!("mfa device arn is not set, cannot get sts token"))?,
            ),
            token_code: Some(mfa),
        };
        let get_session_token_res = sts_client
            .get_session_token(get_session_token)
            .await
            .context("Failed getting STS credentials")?;

        let credentials = get_session_token_res
            .credentials
            .context("Got not credentials")?;

        config.aws_temp_access_key_id = Some(credentials.access_key_id);
        config.aws_temp_secret_access_key = Some(credentials.secret_access_key);
        config.aws_session_token = Some(credentials.session_token);
        config.aws_session_expiration = Some(
            DateTime::parse_from_rfc3339(&credentials.expiration)
                .context("Invalid token expiration format")?,
        );
    }
    config.persist()?;
    Ok(())
}

pub async fn assume_role(
    config: &config::Config,
    client: Arc<HttpClient>,
    role_arn: &String,
) -> Result<Credentials> {
    let cred_provider = StaticProvider::new(
        config.aws_temp_access_key_id.clone().unwrap(),
        config.aws_temp_secret_access_key.clone().unwrap(),
        config.aws_session_token.clone(),
        None,
    );
    let sts_client = StsClient::new_with(client, cred_provider, Region::EuWest1);

    println!("Assuming role {}", role_arn.to_string());

    let assume_role_request = AssumeRoleRequest {
        role_arn: role_arn.to_owned(),
        role_session_name: "dummy".to_owned(),
        ..Default::default()
    };

    let assume_role_res = sts_client
        .assume_role(assume_role_request)
        .await
        .context("Failed assuming role")?;

    let credentials = assume_role_res
        .credentials
        .context("Missing credentials from assume role")?;

    Ok(Credentials {
        aws_access_key: credentials.access_key_id,
        aws_secret_key: credentials.secret_access_key,
        aws_sts_token: credentials.session_token,
    })
}
