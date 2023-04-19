// Copyright (C) Microsoft Corporation. All rights reserved.

use crate::client::error::{io_err, Result};
use azure_storage_blobs::prelude::*;
use bytes::Bytes;
use futures::stream::StreamExt;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressFinish, ProgressStyle};
use std::path::Path;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use url::Url;

/// Upload a file to Azure Blob Storage
pub(crate) async fn blob_upload(mut handle: File, sas: Url) -> Result<()> {
    let size = handle
        .metadata()
        .await
        .map_err(|e| io_err("reading file size", e))?
        .len();

    let block_size = std::cmp::max(1024 * 1024 * 10, size / 50_000);
    let block_size_usize = block_size.try_into()?;

    let style = ProgressStyle::with_template(
        "[{elapsed_precise}] [eta:{eta}] [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec})",
    )?;
    let status = ProgressBar::with_draw_target(Some(size), ProgressDrawTarget::stderr_with_hz(1))
        .with_style(style)
        .with_finish(ProgressFinish::AndLeave);

    let blob_client = BlobClient::from_sas_url(&sas)?;

    let mut block_list = vec![];
    for i in 0..usize::MAX {
        let mut data = Vec::with_capacity(block_size_usize);
        let mut take_handle = handle.take(block_size);
        let read_data = take_handle
            .read_to_end(&mut data)
            .await
            .map_err(|e| io_err("reading block", e))?;
        if read_data == 0 {
            break;
        }
        handle = take_handle.into_inner();
        let id = Bytes::from(format!("{i:032x}"));
        blob_client
            .put_block(id.clone(), data)
            .into_future()
            .await?;
        block_list.push(id);
        status.inc(read_data as u64);
    }

    let blocks = block_list
        .into_iter()
        .map(|x| BlobBlockType::Uncommitted(BlockId::new(x)))
        .collect::<Vec<_>>();
    blob_client
        .put_block_list(BlockList { blocks })
        .into_future()
        .await?;

    Ok(())
}

/// Convert a SAS URL to an Azure Blob Storage `ContainerClient`
pub(crate) fn container_client(container_sas: &Url) -> Result<ContainerClient> {
    let container_client = ContainerClient::from_sas_url(container_sas)?;
    Ok(container_client)
}

/// Convert a container SAS URL to an Azure Blob Storage `BlobClient`
fn blob_client<N>(container_sas: &Url, name: N) -> Result<BlobClient>
where
    N: Into<String>,
{
    let container_client = container_client(container_sas)?;
    let blob_client = container_client.blob_client(name);
    Ok(blob_client)
}

/// Return the contents of a blob
pub(crate) async fn blob_get<N>(container_sas: &Url, name: N) -> Result<Vec<u8>>
where
    N: Into<String>,
{
    let blob_client = blob_client(container_sas, name)?;
    let blob = blob_client.get_content().await?;
    Ok(blob)
}

/// Download the contents of the specified blob to a file with a blob sas URL
pub(crate) async fn blob_download<P>(blob_url: &Url, filename: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let filename = filename.as_ref();
    let blob_client = BlobClient::from_sas_url(blob_url)?;
    let size = blob_client
        .get_properties()
        .await?
        .blob
        .properties
        .content_length;

    let style = ProgressStyle::with_template(
        "[{elapsed_precise}] [eta:{eta}] [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec})",
    )?;
    let status = ProgressBar::with_draw_target(Some(size), ProgressDrawTarget::stderr_with_hz(1))
        .with_style(style)
        .with_finish(ProgressFinish::AndLeave);

    let mut stream = blob_client.get().into_stream();

    let mut file = File::create(filename)
        .await
        .map_err(|e| io_err(format!("creating file: {filename:?}"), e))?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let mut body = chunk.data;

        while let Some(value) = body.next().await {
            let value = value?;
            file.write_all(&value)
                .await
                .map_err(|e| io_err(format!("writing blob: {filename:?}"), e))?;
            status.inc(value.len() as u64);
        }
    }

    Ok(())
}

/// Download the contents of the specified blob to a file
pub(crate) async fn container_blob_download<P, N>(
    container_sas: &Url,
    name: N,
    filename: P,
) -> Result<()>
where
    P: AsRef<Path>,
    N: Into<String>,
{
    let filename = filename.as_ref();
    let blob_client = blob_client(container_sas, name)?;
    let mut stream = blob_client.get().into_stream();

    let mut file = File::create(filename)
        .await
        .map_err(|e| io_err(format!("creating file: {filename:?}"), e))?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let mut body = chunk.data;

        while let Some(value) = body.next().await {
            let value = value?;
            file.write_all(&value)
                .await
                .map_err(|e| io_err(format!("writing blob: {filename:?}"), e))?;
        }
    }

    Ok(())
}
