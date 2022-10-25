// Copyright (C) Microsoft Corporation. All rights reserved.

//! This example illustrates using [Azure Custom Script
//! Extension](https://learn.microsoft.com/en-us/azure/virtual-machines/extensions/custom-script-linux)
//! to launch [AVML](https://github.com/microsoft/avml) to capture memory from a
//! VM in Azure, with the resulting image being uploaded to Project Freta.

use clap::{Parser, Subcommand};
use freta::{argparse::parse_key_val, Client, Error, Image, ImageFormat, Result};
use log::info;
use powershell_script::PsScriptBuilder;
use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// list currently running VMs
    List,
    /// image a single VM
    ImageVm(ImageOpt),
}

#[derive(Parser)]
struct ImageOpt {
    vm_name: String,

    #[clap(long, value_name = "KEY=VALUE", value_parser = parse_key_val::<String, String>, action = clap::ArgAction::Append)]
    /// specify multiple times to include multiple key/value pairs
    tags: Option<Vec<(String, String)>>,

    #[arg(long)]
    /// after the VM is uploaded, monitor the analysis until it's compute
    monitor: bool,
}

#[derive(Deserialize)]
struct Snapshot {
    #[serde(alias = "Id")]
    id: String,
    #[serde(alias = "Path")]
    path: PathBuf,
}

fn run<Q>(query: Q) -> Result<String>
where
    Q: AsRef<str>,
{
    let ps = PsScriptBuilder::new()
        .no_profile(true)
        .non_interactive(true)
        .hidden(true)
        .print_commands(false)
        .build();
    let output = ps
        .run(query.as_ref())
        .map_err(|e| Error::Other("launching powershell failed", format!("{:?}", e)))?;
    if !output.success() {
        return Err(Error::Other(
            "command failed",
            output
                .stderr()
                .or_else(|| output.stdout())
                .unwrap_or_else(|| "unknown error".to_string()),
        ));
    }
    Ok(output.stdout().unwrap_or_default())
}

#[derive(Deserialize, Debug)]
struct Entry {
    #[serde(alias = "VMName")]
    name: String,

    #[serde(alias = "VMId")]
    id: Uuid,
}

#[derive(Deserialize, Debug)]
struct Entries(Vec<Entry>);

fn list_vms() -> Result<Entries> {
    // if there is only one output, we get a single dict.  if there are
    // multiple, we get a list of dicts.
    let out = run("get-vm | select vmname, vmid, state | where state -eq 'running' | select vmname,vmid | convertto-json")?;
    let entries = if let Ok(entry) = serde_json::from_str::<Entry>(&out) {
        Entries(vec![entry])
    } else {
        serde_json::from_str::<Entries>(&out)?
    };

    Ok(entries)
}

fn get_vm_id(vm_name: &str) -> Result<Uuid> {
    for vm in list_vms()?.0 {
        if vm.name == vm_name {
            return Ok(vm.id);
        }
    }
    Err(Error::Other(
        "unable to find running VM",
        vm_name.to_string(),
    ))
}

async fn create_snapshot(
    vm_name: String,
    mut tags: Vec<(String, String)>,
    client: &mut Client,
) -> Result<Image> {
    let vm_id = get_vm_id(&vm_name)?;

    let snapshot_id = Uuid::new_v4();
    info!("creating hyperv snapshot id: {}", snapshot_id);

    run(format!(
        "get-vm -id {} | checkpoint-vm -snapshotname {}",
        vm_id, snapshot_id
    ))?;

    let output = run(format!(
        "get-vm -id {} | get-vmsnapshot -name {} | select id, path | convertto-json",
        vm_id, snapshot_id
    ))?;
    let snapshot: Snapshot = serde_json::from_str(&output)?;
    let path = snapshot
        .path
        .join("Snapshots")
        .join(format!("{}.VMRS", snapshot.id));

    tags.push(("name".to_string(), vm_name.clone()));
    let image = client.images_upload(ImageFormat::Vmrs, tags, path).await?;
    info!("image_id: {}", image.image_id);

    run(format!(
        "get-vm -id {} | get-vmsnapshot -name {} | remove-vmsnapshot",
        vm_id, snapshot_id
    ))?;

    Ok(image)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cmd = Args::parse();

    let mut client = Client::new().await?;

    match cmd.command {
        Commands::List => {
            let vms = list_vms()?;
            for vm in vms.0 {
                info!("{}", vm.name);
            }
        }
        Commands::ImageVm(opts) => {
            let image =
                create_snapshot(opts.vm_name, opts.tags.unwrap_or_default(), &mut client).await?;
            if opts.monitor {
                client.images_monitor(image.image_id).await?;
            }
        }
    }

    Ok(())
}
