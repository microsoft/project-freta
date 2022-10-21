// Copyright (C) Microsoft Corporation. All rights reserved.

/// Freta CLI command line parsing helpers
pub mod argparse;
pub(crate) mod backend;
pub(crate) mod config;
pub(crate) mod error;

use crate::{
    client::{
        backend::{
            azure_blobs::{blob_download, blob_get, blob_upload, container_client},
            Backend,
        },
        error::{Error, Result},
    },
    models::{
        base::{Image, ImageFormat, ImageId, ImageState, OwnerId},
        service::{
            ImageCreate, ImageDeleteResponse, ImageList, ImageReanalyzeResponse, ImageUpdate,
            ImagesListResponse, Info, UserConfig, UserConfigUpdateResponse,
        },
    },
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use log::info;
use std::{collections::BTreeMap, path::Path, pin::Pin, time::Duration};
use tokio::time::sleep;
use url::Url;

fn as_tags<T, K, V>(tags: T) -> BTreeMap<String, String>
where
    T: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    tags.into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect()
}

const IMAGE_MONITOR_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
/// Freta Client
pub struct Client {
    backend: Backend,
}

impl Client {
    /// Create a new client for the Freta service
    pub async fn new() -> Result<Self> {
        let backend = Backend::new().await?;
        Ok(Self { backend })
    }

    /// logout of the service
    pub async fn logout() -> Result<()> {
        Backend::logout().await?;
        Ok(())
    }

    /// Retrieve user configuration settings
    pub async fn user_config_get(&mut self) -> Result<UserConfig> {
        let res = self.backend.get("/api/users", None::<String>).await?;
        Ok(res)
    }

    /// Update user configuration settings
    pub async fn user_config_update(
        &mut self,
        eula_accepted: Option<String>,
        include_samples: bool,
    ) -> Result<UserConfigUpdateResponse> {
        let config = UserConfig {
            eula_accepted,
            include_samples,
        };
        let res = self.backend.post("/api/users", config).await?;
        Ok(res)
    }

    /// Get the latest EULA required to use the service
    pub async fn eula(&mut self) -> Result<Bytes> {
        let res = self.backend.get_raw("/api/eula", None::<String>).await?;
        Ok(res)
    }

    /// Retrieve information about the service
    pub async fn info(&mut self) -> Result<Info> {
        let res = self.backend.get("/api/info", None::<String>).await?;
        Ok(res)
    }

    /// List available images
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use freta::{Client, ImageFormat::Lime, Result};
    /// use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let mut client = Client::new().await?;
    /// let mut stream = client.images_list(None, None, None, true);
    /// while let Some(image) = stream.next().await {
    ///     let image = image?;
    ///     println!("{:?}", image);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn images_list(
        &mut self,
        image_id: Option<ImageId>,
        owner_id: Option<OwnerId>,
        state: Option<ImageState>,
        include_samples: bool,
    ) -> Pin<Box<impl Stream<Item = std::result::Result<Image, crate::Error>> + Send + '_>> {
        let mut image_list = ImageList {
            image_id,
            owner_id,
            state,
            include_samples,
            continuation: None,
        };
        Box::pin(async_stream::try_stream! {
            loop {
                let result: ImagesListResponse = self.backend.get("/api/images", Some(&image_list)).await?;
                for image in result.images {
                    yield image;
                }
                image_list.continuation = result.continuation;
                if image_list.continuation.is_none() {
                    break;
                }
            }
        })
    }

    /// Create a new image entry
    ///
    /// The resulting `Image.image_url` is a time-limited
    /// [SAS URL](https://docs.microsoft.com/azure/storage/common/storage-sas-overview)
    /// that can be used to upload a memory snapshot to Freta via tools such as
    /// [azcopy](https://learn.microsoft.com/en-us/azure/storage/common/storage-ref-azcopy)
    pub async fn images_create<T, K, V>(&mut self, format: ImageFormat, tags: T) -> Result<Image>
    where
        T: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let tags = as_tags(tags);
        let create = ImageCreate { format, tags };
        let res = self.backend.post("/api/images", create).await?;
        Ok(res)
    }

    /// Create and upload an image to Freta
    pub async fn images_upload<P, T, K, V>(
        &mut self,
        format: ImageFormat,
        tags: T,
        path: P,
    ) -> Result<Image>
    where
        P: AsRef<Path>,
        T: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let image = self.images_create(format, tags).await?;

        info!("uploading as image id: {}", image.image_id);

        let image_url = image
            .image_url
            .clone()
            .ok_or(Error::InvalidResponse("missing image_url"))?;
        blob_upload(path, image_url).await?;

        Ok(image)
    }

    /// Get information on an image
    pub async fn images_get(&mut self, image_id: ImageId) -> Result<Image> {
        let res = self
            .backend
            .get(&format!("/api/images/{}", image_id), None::<bool>)
            .await?;
        Ok(res)
    }

    /// Delete an image
    pub async fn images_delete(&mut self, image_id: ImageId) -> Result<ImageDeleteResponse> {
        let res = self
            .backend
            .delete(&format!("/api/images/{}", image_id))
            .await?;
        Ok(res)
    }

    /// Update metadata for an image
    ///
    /// If `tags` is not None, then the tags are overwritten.
    /// If `shareable` is not None, then the shareable value is overwritten.
    pub async fn images_update<T, K, V>(
        &mut self,
        image_id: ImageId,
        tags: Option<T>,
        shareable: Option<bool>,
    ) -> Result<Image>
    where
        T: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let tags = tags.map(as_tags);
        let update = ImageUpdate { tags, shareable };
        let res = self
            .backend
            .post(&format!("/api/images/{}", image_id), update)
            .await?;
        Ok(res)
    }

    /// Reanalyze an image
    pub async fn images_reanalyze(&mut self, image_id: ImageId) -> Result<ImageReanalyzeResponse> {
        let res = self
            .backend
            .patch(&format!("/api/images/{}", image_id), None::<bool>)
            .await?;
        Ok(res)
    }

    /// Get the SAS URL for the Azure Storage container for artifacts extracted
    /// from the image
    async fn artifacts_get_sas(&mut self, image_id: ImageId) -> Result<Url> {
        let image = self.images_get(image_id).await?;
        let image_url = match image.artifacts_url {
            Some(image_url) => image_url,
            None => return Err(Error::InvalidResponse("missing artifacts_url")),
        };

        Ok(image_url)
    }

    /// List the artifacts extracted from the image
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use freta::{Client, ImageFormat::Lime, ImageId, Result};
    /// # use std::str::FromStr;
    /// use futures::StreamExt;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let image_id = ImageId::from_str("a57034e6-7dfe-4b87-a894-e864dd8c8374").unwrap();
    /// let mut client = Client::new().await?;
    /// let mut stream = client.artifacts_list(image_id);
    /// while let Some(name) = stream.next().await {
    ///     let name = name?;
    ///     println!("{}", name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn artifacts_list(
        &mut self,
        image_id: ImageId,
    ) -> Pin<Box<impl Stream<Item = std::result::Result<String, crate::Error>> + Send + '_>> {
        Box::pin(async_stream::try_stream! {
            let container_sas = self.artifacts_get_sas(image_id).await?;
            let container_client = container_client(&container_sas)?;
            let mut stream = container_client.list_blobs().into_stream();

            while let Some(entries) = stream.next().await {
                let entries = entries?;
                let blob_names: Vec<_> = entries.blobs.blobs().map(|b| b.name.clone()).collect();
                for name in blob_names {
                    yield name;
                }
            }
        })
    }

    /// Get an artifact extracted from the image
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use freta::{Client, ImageFormat::Lime, Result, ImageId};
    /// # use std::str::FromStr;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let image_id = ImageId::from_str("a57034e6-7dfe-4b87-a894-e864dd8c8374").unwrap();
    /// let mut client = Client::new().await?;
    /// let report = client.artifacts_get(image_id, "report.json").await?;
    /// # Ok(())
    /// # }
    pub async fn artifacts_get<N>(&mut self, image_id: ImageId, name: N) -> Result<Vec<u8>>
    where
        N: Into<String>,
    {
        let url = self.artifacts_get_sas(image_id).await?;
        let blob = blob_get(&url, name).await?;
        Ok(blob)
    }

    /// Download an artifact extracted from the image to a file
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use freta::{Client, ImageFormat::Lime, Result, ImageId};
    /// # use std::str::FromStr;
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// # let image_id = ImageId::from_str("a57034e6-7dfe-4b87-a894-e864dd8c8374").unwrap();
    /// let mut client = Client::new().await?;
    /// client.artifacts_download(image_id, "report.json", "/tmp/report.json").await?;
    /// # Ok(())
    /// # }
    pub async fn artifacts_download<P, N>(
        &mut self,
        image_id: ImageId,
        name: N,
        output: P,
    ) -> Result<()>
    where
        P: AsRef<Path>,
        N: Into<String>,
    {
        let url = self.artifacts_get_sas(image_id).await?;
        blob_download(&url, name, output).await?;
        Ok(())
    }

    /// Monitor the ongoing state of an image until the analysis has completed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use freta::{Client, ImageFormat::Lime, Result, ImageId};
    /// # #[tokio::main]
    /// # async fn main() -> Result<()> {
    /// let mut client = Client::new().await?;
    /// let image = client.images_create(Lime, [("name", "test image")]).await?;
    /// client.images_monitor(image.image_id).await?;
    /// # Ok(())
    /// # }
    pub async fn images_monitor(&mut self, image_id: ImageId) -> Result<()> {
        let mut image = self.images_get(image_id).await?;
        let mut prev_state = image.state.clone();
        let mut first = true;
        loop {
            if image.state != prev_state || first {
                match image.state {
                    ImageState::Completed => {
                        info!("analysis completed");
                        break;
                    }
                    ImageState::Failed => {
                        if let Some(error) = image.error {
                            return Err(Error::AnalysisFailed(error.into()));
                        }
                        return Err(Error::AnalysisFailed("unknown error".into()));
                    }
                    ImageState::WaitingForUpload
                    | ImageState::ToQueue
                    | ImageState::Queued
                    | ImageState::Running
                    | ImageState::Finalizing
                    | ImageState::Deleting => {
                        info!("{:?}", image.state);
                    }
                }
            }
            sleep(IMAGE_MONITOR_INTERVAL).await;

            prev_state = image.state;
            image = self.images_get(image_id).await?;
            first = false;
        }
        Ok(())
    }
}
