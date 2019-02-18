# Description

Just an example using Rusoto AWS Rust library through a http proxy. It uses the STS, ECS and credentials AWS services.

The example here supposes that your organization has multiple AWS accounts and MFA authentication is required.

The program just list the ECS clusters.

The first execution will ask for aws mfa device ARN, optionally the AWS access and secret keys and persist them.

MFA is also asked to get a STS session token and the token is persisted. If it expires MFA is asked again.

To run just do

```
cargo run -- "arn:aws:iam::123456789:role/MyRoleInTheOrganization"
```

# Notes

I am just learning Rust so the code is not surely idiomatic Rust.

Error management in my example is also not consistent

