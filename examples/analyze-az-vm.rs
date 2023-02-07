// Copyright (C) Microsoft Corporation. All rights reserved.

//! This example illustrates using [Azure Custom Script
//! Extension](https://learn.microsoft.com/en-us/azure/virtual-machines/extensions/custom-script-linux)
//! to launch [AVML](https://github.com/microsoft/avml) to capture memory from a
//! VM in Azure, with the resulting image being uploaded to Project Freta.

use azure_identity::DefaultAzureCredential;
use azure_mgmt_compute::models::{
    ResourceWithOptionalLocation, VirtualMachineExtension, VirtualMachineExtensionProperties,
};
use clap::Parser;
use freta::{argparse::parse_key_val, Client, Error, ImageFormat, Result};
use log::info;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

// https://learn.microsoft.com/en-us/azure/virtual-machines/extensions/custom-script-linux#extension-schema
const EXTENSION_PUBLISHER: &str = "Microsoft.Azure.Extensions";
const EXTENSION_NAME: &str = "CustomScript";
const EXTENSION_VERSION: &str = "2.1";

#[derive(Parser)]
struct Args {
    subscription_id: String,
    group: String,
    vm_name: String,

    #[clap(long, value_name = "KEY=VALUE", value_parser = parse_key_val::<String, String>, action = clap::ArgAction::Append)]
    /// specify multiple times to include multiple key/value pairs
    tags: Option<Vec<(String, String)>>,

    #[arg(long)]
    /// after the VM is uploaded, monitor the analysis until it's compute
    monitor: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cmd = Args::parse();

    let mut client = Client::new().await?;

    let creds = Arc::new(DefaultAzureCredential::default());
    let compute_client = azure_mgmt_compute::Client::builder(creds).build();

    let vm = compute_client
        .virtual_machines_client()
        .get(&cmd.group, &cmd.vm_name, &cmd.subscription_id)
        .into_future()
        .await?;

    let mut tags = cmd.tags.unwrap_or_default();
    tags.push(("name".to_string(), cmd.vm_name.clone()));
    tags.push(("group".to_string(), cmd.group.clone()));

    let image = client.images_create(ImageFormat::Lime, tags).await?;

    info!("image: {}", image.image_id);

    let image_url = image
        .image_url
        .clone()
        .ok_or(Error::InvalidResponse("missing image_url"))?;

    let settings = json!({
        "fileUris": [
            "https://github.com/microsoft/avml/releases/download/v0.10.0/avml"
        ]
    });

    let protected_settings = json!({
        "commandToExecute": format!("./avml /root/{}.lime --compress --delete --sas-url '{image_url}'", Uuid::new_v4()),
    });

    let extension_parameters = VirtualMachineExtension {
        resource_with_optional_location: ResourceWithOptionalLocation {
            location: Some(vm.resource.location),
            ..Default::default()
        },
        properties: Some(VirtualMachineExtensionProperties {
            publisher: Some(EXTENSION_PUBLISHER.to_string()),
            type_: Some(EXTENSION_NAME.to_string()),
            type_handler_version: Some(EXTENSION_VERSION.to_string()),
            settings: Some(settings),
            protected_settings: Some(protected_settings),
            ..Default::default()
        }),
    };

    compute_client
        .virtual_machine_extensions_client()
        .create_or_update(
            &cmd.group,
            &cmd.vm_name,
            EXTENSION_PUBLISHER,
            extension_parameters,
            &cmd.subscription_id,
        )
        .into_future()
        .await?;

    if cmd.monitor {
        client.images_monitor(image.image_id).await?;
    }
    Ok(())
}
