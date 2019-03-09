error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Network(::hyper_tls::Error);
        Json(::serde_json::error::Error);
        InvalidUri(::hyper::http::uri::InvalidUri);
        AwsEcsDescribeTaskDefinitionError(::rusoto_ecs::DescribeTaskDefinitionError);
        AwsEcsListServicesError(::rusoto_ecs::ListServicesError);
        AwsEcsListClustersError(::rusoto_ecs::ListClustersError);
        AwsEcsDescribeServicesError(::rusoto_ecs::DescribeServicesError);
    }
    errors {
        InvalidConfig(t: String) {
            description("invalid configuration")
            display("invalid configuration '{}'", t)
        }
    }
}
