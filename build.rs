// Copyright (C) Microsoft Corporation. All rights reserved.

use cargo_metadata::{CargoOpt, Metadata, MetadataCommand};
use project_root::get_project_root;
use serde::Serialize;
use std::{env, error::Error, fs::write, path::PathBuf, result::Result};

#[derive(Serialize, Debug)]
struct Package<'a> {
    name: &'a str,
    version: String,
    license: &'a str,
}

/// get the list of dependencies of this crate
fn get_dependencies(metadata: &Metadata) -> Vec<&str> {
    metadata
        .packages
        .iter()
        .find(|package| package.name == env!("CARGO_PKG_NAME"))
        .map(|x| x.dependencies.iter().map(|y| y.name.as_str()).collect())
        .unwrap_or_default()
}

fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "cargo:rerun-if-changed={}",
        get_project_root()?.join("Cargo.lock").display()
    );

    let metadata = MetadataCommand::new()
        .features(CargoOpt::AllFeatures)
        .exec()?;

    let dependencies = get_dependencies(&metadata);

    let mut licenses = vec![];
    for package in &metadata.packages {
        if !dependencies.contains(&package.name.as_ref()) {
            continue;
        }

        // skip crates maintained by this team
        if package.authors == ["project-freta@microsoft.com"] {
            continue;
        }

        let Some(license) = package.license.as_ref() else {
            return Err(format!("package {} has no license", package.name).into());
        };

        licenses.push(Package {
            name: package.name.as_ref(),
            version: package.version.to_string(),
            license,
        });
    }

    let as_string = serde_json::to_string_pretty(&licenses)?;
    let path = PathBuf::from(env::var("OUT_DIR")?).join("licenses.json");
    write(path, as_string)?;
    Ok(())
}
