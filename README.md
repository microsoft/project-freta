# Project Freta

## Summary

The `Freta SDK` enables access to [Project Freta](https://freta.microsoft.com), a service used to inspect volatile memory images.

Included in this library is a utility, `freta`, which provides command line access to the [Project Freta](https://freta.microsoft.com) service.

## Documentation

* [Service Documentation](https://learn.microsoft.com/en-us/security/research/project-freta/) contains information on how to use the Service and details to the information exposed by the Freta analysis.
* [CLI Reference](https://learn.microsoft.com/en-us/security/research/project-freta/api/cli-reference) contains information on how to use the Freta CLI to interact with the Freta service.
* [Module Documentation](https://docs.rs/freta/latest) describes the APIs and data structures  to programmatically interact with the Freta service.
* [Examples](https://github.com/microsoft/project-freta/tree/main/examples) demonstrates how to build upon the SDK to automate Freta.

## Installing

```
cargo install freta
```

## Building

The Freta client is written in [Rust](https://www.rust-lang.org/) and requires Rust 1.64.0 (stable) or newer.

To build freta:

```
$ git clone https://github.com/microsoft/project-freta
$ cd project-freta
$ cargo build --release
$ ./target/release/freta --version
0.9.0
```

## Using the Client

```
$ freta info
{
  "api_version": "0.7.2",
  "models_version": "0.9.0",
  "current_eula": "993C44214D3E5D0EEB92679E41FC0C4D69DA9C37EF97988FB724C7B2493695BB",
  "formats": [
    "vmrs",
    "raw",
    "lime",
    "core",
    "avmh"
  ]
}
$ freta images upload ~/projects/samples/centos-6-2.6.32-754.17.1.el6/OpenLogic\:CentOS\:6.10\:latest.lime
[2022-10-20T17:37:39Z INFO  freta] uploading as image id: 78f6bdc7-31ce-4877-a67e-f7137db248bd
206.99 MiB (25.26 MiB/s)
$ freta images list
[
    {
      "last_updated": "2022-10-20T17:38:29.7311842Z",
      "owner_id": "72f988bf-86f1-41af-91ab-2d7cd011db47_09731d72-f8a6-463c-9efe-ca0bedfd82ae",
      "image_id": "78f6bdc7-31ce-4877-a67e-f7137db248bd",
      "state": "completed",
      "format": "lime",
      "tags": {},
      "shareable": false
    },    
]
```

## Using Freta with automated tools

Instead of logging in as a user to the Freta service, automated tools can use the [Azure service principals](https://learn.microsoft.com/en-us/cli/azure/create-an-azure-service-principal-azure-cli) for authentication.

First, create a service principal:
```
$ az ad sp create-for-rbac -n YOUR_SERVICE_PRINCIPAL_NAME_HERE
{
  "appId": "00000000-0000-0000-0000-000000000000",
  "displayName": "YOUR_SERVICE_PRINCIPAL_NAME_HERE",
  "password": "REDACTED_PASSWORD_HERE",
  "tenant": "REDACTED_TENANT_HERE",
}
```

Then configure the freta client to use the newly created service principal, use the `appId` from the output of the previous command as the `--client-id` and the `password` as the `--client-secret`, and use the `tenant` as the `--tenant-id`:
```
$ freta config --client-id APP_ID_HERE --client-secret PASSWORD_HERE --tenant-id TENANT_HERE
[2023-01-04T19:43:34Z INFO  freta] config saved: Config { api url: "https://freta.microsoft.com/", client id: "00000000-0000-0000-0000-000000000000", tenant id: "REDACTED_TENANT_HERE", client secret: "[redacted]", scope: "api://a934fc14-92d7-4127-aecd-bddab35935da/.default" }
```

Before using the service, the service principal must agree to the Freta EULA. This can be done by running the following command:
```
$ freta eula accept
```

From here, the client can be used in an automated fashion.

# Contributing

This project welcomes contributions and suggestions. Most contributions require you to
agree to a Contributor License Agreement (CLA) declaring that you have the right to,
and actually do, grant us the rights to use your contribution. For details, visit
https://cla.microsoft.com.

When you submit a pull request, a CLA-bot will automatically determine whether you need
to provide a CLA and decorate the PR appropriately (e.g., label, comment). Simply follow the
instructions provided by the bot. You will only need to do this once across all repositories using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/)
or contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

# Reporting Security Issues

Security issues and bugs should be reported privately, via email, to the Microsoft Security
Response Center (MSRC) at [secure@microsoft.com](mailto:secure@microsoft.com). You should
receive a response within 24 hours. If for some reason you do not, please follow up via
email to ensure we received your original message. Further information, including the
[MSRC PGP](https://technet.microsoft.com/en-us/security/dn606155) key, can be found in
the [Security TechCenter](https://technet.microsoft.com/en-us/security/default).
