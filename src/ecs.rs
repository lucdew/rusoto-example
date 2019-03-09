use std::collections::HashMap;
use std::vec::Vec;

use client::HttpConnector;
use credentials::Credentials;
use errors::*;

use futures::future::{join_all, loop_fn, ok, Either, Loop};
use futures::Future;

use rusoto_core::region::Region;
use rusoto_core::request;
use rusoto_credential::StaticProvider;

use rusoto_ecs::{
    DescribeServicesRequest, DescribeTaskDefinitionRequest, Ecs, EcsClient, ListClustersRequest,
    ListServicesRequest,
};

pub fn get_all_services<'a>(
    ecs_client: &'a EcsClient,
    cluster_name: String,
) -> impl Future<Item = Vec<String>, Error = Error> + 'a {
    loop_fn((Vec::new(), None), move |(mut services, next_token)| {
        let list_services_req = ListServicesRequest {
            max_results: None,
            next_token: next_token,
            cluster: Some(cluster_name.clone()),
            launch_type: None,
            scheduling_strategy: None,
        };

        ecs_client
            .list_services(list_services_req)
            .and_then(|list_services_res| {
                if list_services_res.service_arns.is_some() {
                    services.extend(list_services_res.service_arns.unwrap())
                }
                if list_services_res.next_token.is_none() {
                    Ok(Loop::Break(services))
                } else {
                    Ok(Loop::Continue((services, list_services_res.next_token)))
                }
            })
    })
    .map_err(|err| err.into())
}

pub fn get_images_of_task_definition<'a>(
    ecs_client: &'a EcsClient,
    task_definitions: Vec<String>,
) -> impl Future<Item = Vec<String>, Error = Error> + 'a {
    let get_images_futures = task_definitions.into_iter().map(move |td| {
        let task_definition_req = DescribeTaskDefinitionRequest {
            task_definition: td,
        };
        ecs_client
            .describe_task_definition(task_definition_req)
            .map(|task_definition_res| {
                task_definition_res
                    .task_definition
                    .and_then(|td| td.container_definitions)
                    .and_then(|cds| cds.last().cloned())
                    .and_then(|cd| cd.image)
                    .and_then(|image| Some(image))
            })
    });

    join_all(get_images_futures)
        .map(|found_images| found_images.into_iter().flatten().collect())
        .map_err(|err| err.into())
}

fn get_images_of_services<'a>(
    ecs_client: &'a EcsClient,
    service_arns: Vec<String>,
    cluster_name: String,
) -> impl Future<Item = Vec<String>, Error = Error> + 'a {
    let describe_services_req = DescribeServicesRequest {
        cluster: Some(cluster_name.clone()),
        services: service_arns,
    };

    ecs_client
        .describe_services(describe_services_req)
        .map_err(|err| err.into())
        .and_then(move |describe_services_res| {
            debug!("Getting images for cluster {}", cluster_name);
            if describe_services_res.services.is_none() {
                return Either::A(ok(Vec::new()));
            }
            let task_definitions = describe_services_res
                .services
                .unwrap()
                .into_iter()
                .map(|service| service.task_definition)
                .flatten()
                .collect();

            debug!("Got task definitions {:?}", task_definitions);

            Either::B(
                get_images_of_task_definition(ecs_client, task_definitions)
                    .and_then(|images| ok(images)),
            )
        })
}

pub fn get_images_of_a_cluster<'a>(
    ecs_client: &'a EcsClient,
    cluster_name: String,
) -> impl Future<Item = (String, Vec<String>), Error = Error> + 'a {
    let cluster_name_clone = cluster_name.clone();
    get_all_services(&ecs_client, cluster_name.clone()).and_then(move |all_services| {
        let all_services_len = all_services.len();
        let mut services_partions = Vec::new();
        let partition_num = (all_services_len / 10) + 1;
        for i in 0..partition_num {
            let last_idx = if i == partition_num - 1 {
                all_services_len
            } else {
                10 * i + 10
            };
            let partition = all_services[i * 10..last_idx].to_vec();
            services_partions.push(partition);
        }
        join_all(services_partions.into_iter().map(move |services| {
            get_images_of_services(ecs_client, services, cluster_name.clone())
        }))
        .map(move |images_partitions| {
            (
                cluster_name_clone,
                images_partitions.into_iter().flatten().collect(),
            )
        })
    })
}

pub fn get_clusters<'a>(
    ecs_client: &'a EcsClient,
) -> impl Future<Item = Vec<String>, Error = Error> + 'a {
    loop_fn((Vec::new(), None), move |(mut clusters, next_token)| {
        let list_clusters_req = ListClustersRequest {
            max_results: None,
            next_token: next_token,
        };

        ecs_client
            .list_clusters(list_clusters_req)
            .and_then(|list_clusters_res| {
                if list_clusters_res.cluster_arns.is_some() {
                    clusters.extend(list_clusters_res.cluster_arns.unwrap());
                }
                if list_clusters_res.next_token.is_none() {
                    Ok(Loop::Break(clusters))
                } else {
                    Ok(Loop::Continue((clusters, list_clusters_res.next_token)))
                }
            })
    })
    .map_err(|err| err.into())
}

pub fn get_images_of_clusters<'a>(
    ecs_client: &'a EcsClient,
) -> impl Future<Item = HashMap<String, Vec<String>>, Error = Error> + 'a {
    get_clusters(&ecs_client).and_then(move |clusters_arn| {
        debug!("Got clusters {:?}", clusters_arn);

        join_all(
            clusters_arn
                .into_iter()
                .map(move |cluster_arn| get_images_of_a_cluster(&ecs_client, cluster_arn)),
        )
        .map(|cluster_images_tuples| {
            let mut res = HashMap::new();
            for cluster_images_tuple in cluster_images_tuples {
                let (cluster_name, images) = cluster_images_tuple;
                res.insert(cluster_name, images);
            }
            return res;
        })
    })
}

pub fn build_ecs_client(connector: HttpConnector, creds: Credentials) -> EcsClient {
    let client = request::HttpClient::from_connector(connector.clone());
    let cred_provider = StaticProvider::new(
        creds.aws_access_key,
        creds.aws_secret_key,
        Some(creds.aws_sts_token),
        None,
    );
    EcsClient::new_with(client, cred_provider, Region::EuWest1)
}
