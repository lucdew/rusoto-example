# Description

Just an example using [Rusoto](https://rusoto.github.io/rusoto/rusoto_core/index.html) AWS Rust library through a http proxy and [Tokio](https://github.com/tokio-rs/tokio) async runtime.

It uses the STS, ECS and credentials AWS services.

The example here supposes that your organization has multiple AWS accounts and MFA authentication is required.

The program list the lastest ECS images in all ECS clusters.

The first execution will ask for aws mfa device ARN, optionally the AWS access and secret keys and persist them.

MFA is also asked to get a STS session token and the token is persisted. If it expires MFA is asked again.

To run just do

```
cargo run -- "arn:aws:iam::123456789:role/MyRoleInTheOrganization"
```

