// Copyright (C) Microsoft Corporation. All rights reserved.

pub mod argparse;
pub(crate) mod backend;
pub mod config;
pub mod error;

use crate::{
    client::{
        backend::{
            azure_blobs::{blob_download, blob_get, blob_upload, container_list},
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
use log::info;
use std::{collections::BTreeMap, path::Path, time::Duration};
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

pub struct Client {
    backend: Backend,
}

impl Client {
    pub async fn new() -> Result<Self> {
        let backend = Backend::new().await?;
        Ok(Self { backend })
    }

    pub async fn logout() -> Result<()> {
        Backend::logout().await?;
        Ok(())
    }

    pub async fn user_config_get(&mut self) -> Result<UserConfig> {
        let res = self.backend.get("/api/users", None::<String>).await?;
        Ok(res)
    }

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

    pub async fn eula(&mut self) -> Result<Bytes> {
        let res = self.backend.get_raw("/api/eula", None::<String>).await?;
        Ok(res)
    }

    pub async fn info(&mut self) -> Result<Info> {
        let res = self.backend.get("/api/info", None::<String>).await?;
        Ok(res)
    }

    pub async fn images_list(
        &mut self,
        image_id: Option<ImageId>,
        owner_id: Option<OwnerId>,
        state: Option<ImageState>,
        include_samples: bool,
        continuation: Option<String>,
    ) -> Result<ImagesListResponse> {
        let image_list = ImageList {
            image_id,
            owner_id,
            state,
            include_samples,
            continuation,
        };
        let res = self.backend.get("/api/images", Some(image_list)).await?;
        Ok(res)
    }

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

    pub async fn images_get(&mut self, image_id: ImageId) -> Result<Image> {
        let res = self
            .backend
            .get(&format!("/api/images/{}", image_id), None::<bool>)
            .await?;
        Ok(res)
    }

    pub async fn images_delete(&mut self, image_id: ImageId) -> Result<ImageDeleteResponse> {
        let res = self
            .backend
            .delete(&format!("/api/images/{}", image_id))
            .await?;
        Ok(res)
    }

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

    pub async fn images_reanalyze(&mut self, image_id: ImageId) -> Result<ImageReanalyzeResponse> {
        let res = self
            .backend
            .patch(&format!("/api/images/{}", image_id), None::<bool>)
            .await?;
        Ok(res)
    }

    async fn artifacts_get_sas(&mut self, image_id: ImageId) -> Result<Url> {
        let image = self.images_get(image_id).await?;
        let image_url = match image.artifacts_url {
            Some(image_url) => image_url,
            None => return Err(Error::InvalidResponse("missing artifacts_url")),
        };

        Ok(image_url)
    }

    pub async fn artifacts_list(&mut self, image_id: ImageId) -> Result<Vec<String>> {
        let url = self.artifacts_get_sas(image_id).await?;
        let blobs = container_list(&url).await?;
        Ok(blobs)
    }

    pub async fn artifacts_get<N>(&mut self, image_id: ImageId, name: N) -> Result<Vec<u8>>
    where
        N: Into<String>,
    {
        let url = self.artifacts_get_sas(image_id).await?;
        let blob = blob_get(&url, name).await?;
        Ok(blob)
    }

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
