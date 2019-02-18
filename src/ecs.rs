use ::errors::*;
use ::client::HttpConnector;
use ::credentials::Credentials;

use rusoto_core::request;
use rusoto_core::region::Region;
use rusoto_credential::StaticProvider;

use rusoto_ecs::{EcsClient,Ecs, ListClustersRequest};

pub fn get_clusters(connector: HttpConnector, credentials: Credentials) -> Result<()> {

        let cred_provider = StaticProvider::new(
            credentials.aws_access_key,
            credentials.aws_secret_key,
            Some(credentials.aws_sts_token),
            None,
        );

        let client = request::HttpClient::from_connector(connector);

        let ecs_client = EcsClient::new_with(client, cred_provider, Region::EuWest1);

        let list_clusters_req = ListClustersRequest {
            max_results: None,
            next_token: None,
        };

        let list_clusters_res =  ecs_client.list_clusters(list_clusters_req).sync().chain_err(|| "Failed listing clusters")?;

        println!("Got clusters {:?}",list_clusters_res);


        Ok(())

}
