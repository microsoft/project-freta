// Copyright (C) Microsoft Corporation. All rights reserved.

use crate::client::error::{Error, Result};
use azure_storage::prelude::*;
use azure_storage_blobs::prelude::*;
use bytes::Bytes;
use futures::stream::StreamExt;
use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};
use log::debug;
use std::path::Path;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use url::Url;

pub(crate) async fn blob_upload<P>(filename: P, sas: Url) -> Result<()>
where
    P: AsRef<Path>,
{
    let filename = filename.as_ref();

    debug!("uploading {}", filename.display());
    let mut handle = File::open(filename).await?;
    let size = handle.metadata().await?.len();

    let block_size = std::cmp::max(1024 * 1024 * 10, size / 50_000);
    let block_size_usize = block_size.try_into()?;

    let sas: SasToken = sas.try_into()?;
    let style = ProgressStyle::with_template(
        "[{elapsed_precise}] [{wide_bar}] {bytes}/{total_bytes} ({bytes_per_sec})",
    )?;
    let status = ProgressBar::new(size)
        .with_style(style)
        .with_finish(ProgressFinish::AndLeave);

    let blob_name = sas
        .path
        .as_ref()
        .ok_or(Error::InvalidSas("missing blob path"))?;

    let credentials = StorageCredentials::sas_token(&sas.token)?;
    let blob_client = BlobServiceClient::new(&sas.account, credentials)
        .container_client(&sas.container)
        .blob_client(blob_name);

    let mut block_list = vec![];
    for i in 0..usize::MAX {
        let mut data = Vec::with_capacity(block_size_usize);
        let mut take_handle = handle.take(block_size);
        let read_data = take_handle.read_to_end(&mut data).await?;
        if read_data == 0 {
            break;
        }
        handle = take_handle.into_inner();
        let id = Bytes::from(format!("{:032x}", i));
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
    let block_list = BlockList { blocks };
    blob_client.put_block_list(block_list).into_future().await?;

    Ok(())
}

pub(crate) fn container_client(container_sas: &Url) -> Result<ContainerClient> {
    let sas: SasToken = container_sas.clone().try_into()?;
    let credentials = StorageCredentials::sas_token(sas.token)?;
    let container_client =
        BlobServiceClient::new(sas.account, credentials).container_client(sas.container);
    Ok(container_client)
}

fn blob_client<N>(container_sas: &Url, name: N) -> Result<BlobClient>
where
    N: Into<String>,
{
    let container_client = container_client(container_sas)?;
    let blob_client = container_client.blob_client(name);
    Ok(blob_client)
}

pub(crate) async fn blob_get<N>(container_sas: &Url, name: N) -> Result<Vec<u8>>
where
    N: Into<String>,
{
    let blob_client = blob_client(container_sas, name)?;
    let blob = blob_client.get_content().await?;
    Ok(blob)
}

pub(crate) async fn blob_download<P, N>(container_sas: &Url, name: N, filename: P) -> Result<()>
where
    P: AsRef<Path>,
    N: Into<String>,
{
    let filename = filename.as_ref();
    let blob_client = blob_client(container_sas, name)?;
    let mut stream = blob_client.get().into_stream();

    let mut file = File::create(filename).await?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let mut body = chunk.data;

        while let Some(value) = body.next().await {
            let value = value?;
            file.write_all(&value).await?;
        }
    }

    Ok(())
}

struct SasToken {
    account: String,
    container: String,
    path: Option<String>,
    token: String,
}

impl TryFrom<Url> for SasToken {
    type Error = Error;

    fn try_from(url: Url) -> Result<Self> {
        let account = url
            .host_str()
            .ok_or(Error::InvalidSas("missing host)"))?
            .split_terminator('.')
            .next()
            .ok_or(Error::InvalidSas("missing account name"))?
            .to_string();

        let token = url
            .query()
            .ok_or(Error::InvalidSas("missing token"))?
            .to_string();

        let path = url.path();
        let mut v: Vec<&str> = path.split_terminator('/').collect();
        v.remove(0);
        let container = v.remove(0).to_string();
        let path = v.join("/");

        let path = if path.is_empty() { None } else { Some(path) };

        Ok(Self {
            account,
            container,
            path,
            token,
        })
    }
}
