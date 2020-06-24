# Description

Just an example using [Rusoto](https://rusoto.github.io/rusoto/rusoto_core/index.html) AWS Rust library through a http proxy and [Tokio](https://github.com/tokio-rs/tokio) async runtime.

It uses the STS, ECS and credentials AWS services.

The example here supposes that your organization has multiple AWS accounts and MFA authentication is required.

The program list the lastest ECS images in all ECS clusters.

The first execution, the following configuration will be asked:
* If AWS credentials must be retrieved from the ``$HOME/aws/.credentials`` file
** If not, the an AWS access key and secret key will be asked
** If a STS token is not detected in the ``$HOME/aws/.credentials``  file, a MFA device ARN will be asked. And everytime the tool is run a MFA token will be asked to generate a STS token if the previously generated is non-existent or expired (with the related temporary access key and secret)
** It a STS token is detected in the ``$HOME/aws/.credentials`` file, it will assume it is still valid
* Optional roles name/arn couples the tool will use them to retrieve the list of clusters and ECS images using the assumed roles 

The configuration is stored in ``$HOME/.awsManager.json``


To run just do

```
cargo run -- ""
```

Using a specific role arn:
```
cargo run -- "-a arn:aws:iam::123456789:role/MyRoleInTheOrganization"
```
Using a specific role from the configuration filtering by its name from the configuration :
```
cargo run -- "-r dev"
```


Currently it outputs the image name prefixed by the task definition name for each image
