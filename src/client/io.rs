// Copyright (C) Microsoft Corporation. All rights reserved.

use crate::{client::error::io_err, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use tokio::fs;

/// Read and deserialize a JSON file
pub(crate) async fn read_json<P, S>(path: P) -> Result<S>
where
    P: AsRef<Path>,
    S: DeserializeOwned,
{
    let path = path.as_ref();
    let contents = fs::read_to_string(path)
        .await
        .map_err(|e| io_err(format!("reading json file: {path:?}"), e))?;
    let result = serde_json::from_str(&contents)?;
    Ok(result)
}

/// Serialize and write a JSON file
pub(crate) async fn write_json<P, S>(path: P, data: S) -> Result<()>
where
    P: AsRef<Path>,
    S: Serialize,
{
    let path = path.as_ref();
    let contents = serde_json::to_string_pretty(&data)?;
    fs::write(path, contents)
        .await
        .map_err(|e| io_err(format!("writing config: {path:?}"), e))?;
    Ok(())
}

/// Recursively creates a directory and all of its parent components if they are missing.
pub(crate) async fn create_dir_all<P>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    fs::create_dir_all(path)
        .await
        .map_err(|e| io_err(format!("creating directory: {path:?}"), e))
}

/// Removes a file from the filesystem.
pub(crate) async fn remove_file<P>(path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    fs::remove_file(path)
        .await
        .map_err(|e| io_err(format!("removing file: {path:?}"), e))
}

/// Open a file from the filesystem.
pub(crate) async fn open_file<P>(path: P) -> Result<fs::File>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    fs::File::open(path)
        .await
        .map_err(|e| io_err(format!("opening file: {path:?}"), e))
}
