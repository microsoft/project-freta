// Copyright (C) Microsoft Corporation. All rights reserved.

/// Freta CLI command line parsing helpers
pub mod argparse;
/// HTTP client used by the client
pub(crate) mod backend;
/// client config
pub(crate) mod config;
/// client error types
pub(crate) mod error;
/// internal IO wrappers
pub(crate) mod io;

use crate::{
    client::{
        backend::{
            azure_blobs::{
                blob_download, blob_get, blob_upload, container_blob_download, container_client,
            },
            Backend,
        },
        config::Config,
        error::{Error, Result},
        io::open_file,
    },
    models::{
        base::{Image, ImageFormat, ImageId, ImageState, OwnerId},
        service::{
            ImageCreate, ImageDeleteResponse, ImageList, ImageReanalyzeResponse, ImageUpdate,
            ImagesListResponse, Info, UserConfig, UserConfigUpdateResponse,
        },
        webhooks::{
            service::{
                WebhookBoolResponse, WebhookEventReplayRequest, WebhookLogListRequest,
                WebhookLogListResponse, WebhookSubmit, WebhooksListRequest, WebhooksListResponse,
            },
            Webhook, WebhookEvent, WebhookEventId, WebhookEventType, WebhookId, WebhookLog,
        },
    },
    Secret,
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    pin::Pin,
    time::Duration,
};
use tokio::time::sleep;
use tracing::{debug, info};
use url::Url;

/// convert an `Iterator` of key/value pairs into a `BTreeMap`
///
/// Useful for turning `[("key", "value")]` into `BTreeMap` of `{ "key": "value" }`
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

/// interval for polling image status
const IMAGE_MONITOR_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug)]
/// Freta Client
pub struct Client {
    /// Backend client
    backend: Backend,
}

impl Client {
    /// Create a new client for the Freta service
    ///
    /// # Errors
    ///
    /// This function will return an error if creating the backend REST API
    /// client fails
    pub async fn new() -> Result<Self> {
        Self::with_config(Config::load().await?).await
    }

    /// Create a new client for the Freta service with a configuration
    ///
    /// # Errors
    ///
    /// This function will return an error if creating the backend REST API
    /// client fails
    pub async fn with_config(config: Config) -> Result<Self> {
        let backend = Backend::new(config).await?;
        Ok(Self { backend })
    }

    /// logout of the service
    ///
    /// # Errors
    /// This function will return an error if deleting the authentication cache
    /// fails
    pub async fn logout() -> Result<()> {
        Backend::logout().await?;
        Ok(())
    }

    /// Retrieve user configuration settings
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to get their configuration
    pub async fn user_config_get(&self) -> Result<UserConfig> {
        let res = self.backend.get("/api/users", None::<String>).await?;
        Ok(res)
    }

    /// Update user configuration settings
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to update their configuration
    pub async fn user_config_update(
        &self,
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
    ///
    /// Note, all API requests to the service will return the EULA as part of
    /// the error in the HTTP Error response if the EULA has not been accepted.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    pub async fn eula(&self) -> Result<Bytes> {
        let res = self.backend.get_raw("/api/eula", None::<String>).await?;
        Ok(res)
    }

    /// Retrieve information about the service
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to get the service information
    pub async fn info(&self) -> Result<Info> {
        let res = self.backend.get("/api/info", None::<String>).await?;
        Ok(res)
    }

    /// List available images
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use futures::StreamExt;
    /// # use freta::{Client, Result};
    /// # async fn example(client: Client) -> Result<()> {
    /// let mut stream = client.images_list(None, None, None, true);
    /// while let Some(image) = stream.next().await {
    ///     let image = image?;
    ///     println!("{image:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission
    pub fn images_list(
        &self,
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
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to create images.
    pub async fn images_create<T, K, V>(&self, format: ImageFormat, tags: T) -> Result<Image>
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
    ///
    /// # Errors
    ///
    /// This function will return an error in the following cases:
    /// 1. Creating the image in Freta fails
    /// 2. Uploading the blob to Azure Storage fails
    pub async fn images_upload<P, T, K, V>(
        &self,
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
        debug!("uploading {}", path.as_ref().display());
        let handle = open_file(path).await?;

        let image = self.images_create(format, tags).await?;

        info!("uploading as image id: {}", image.image_id);

        let image_url = image.image_url.clone().ok_or(Error::InvalidResponse(
            "missing image_url from the response",
        ))?;
        blob_upload(handle, image_url).await?;

        Ok(image)
    }

    /// Get information on an image
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to read the specified image
    pub async fn images_get(&self, image_id: ImageId) -> Result<Image> {
        let res = self
            .backend
            .get(&format!("/api/images/{image_id}"), None::<bool>)
            .await?;
        Ok(res)
    }

    /// Delete an image
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to delete the specified image
    pub async fn images_delete(&self, image_id: ImageId) -> Result<ImageDeleteResponse> {
        let res = self
            .backend
            .delete(&format!("/api/images/{image_id}"))
            .await?;
        Ok(res)
    }

    /// Update metadata for an image
    ///
    /// If `tags` is not None, then the tags are overwritten.
    /// If `shareable` is not None, then the shareable value is overwritten.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to update metadata for the specified image
    pub async fn images_update<T, K, V>(
        &self,
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
            .post(&format!("/api/images/{image_id}"), update)
            .await?;
        Ok(res)
    }

    /// Reanalyze an image
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to reanalyze the specified image
    pub async fn images_reanalyze(&self, image_id: ImageId) -> Result<ImageReanalyzeResponse> {
        let res = self
            .backend
            .patch(&format!("/api/images/{image_id}"), None::<bool>)
            .await?;
        Ok(res)
    }

    /// Download an image to a file
    ///
    /// NOTE: The service only allows downloading images that have been analyzed
    /// successfully.
    ///
    /// # Errors
    ///
    /// This function will return an error in the follow cases:
    /// 1. The user does not have permission to access the specified image
    /// 2. The image was not successfully analyzed
    /// 3. Downloading the image fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use freta::{Client, Result, ImageId};
    /// # async fn example(client: Client, image_id: ImageId) -> Result<()> {
    /// client.images_download(image_id, "/tmp/image.lime").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn images_download<P>(&self, image_id: ImageId, output: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let image = self.images_monitor(image_id).await?;
        let Some(image_url) = image.image_url else {
            return Err(Error::InvalidResponse(
                "service did not provide image_url in the response"
            ))
        };
        blob_download(&image_url, output).await?;
        Ok(())
    }

    /// Get the SAS URL for the Azure Storage container for artifacts extracted
    /// from the image
    ///
    /// # Errors
    ///
    /// This function will return an error in the follow cases:
    /// 1. Getting the image metadata from the service fails
    /// 2. The image metadata in the service is missing `artifacts_url` which
    ///    should always be returned when getting the metadata for a single
    ///    image.
    async fn artifacts_get_sas(&self, image_id: ImageId) -> Result<Url> {
        let image = self.images_monitor(image_id).await?;
        let Some(image_url) = image.artifacts_url else {
            return Err(Error::InvalidResponse(
                "missing artifacts_url from the response",
            ))
        };

        Ok(image_url)
    }

    /// List the artifacts extracted from the image
    ///
    /// # Errors
    ///
    /// This function will return an error in the follow cases:
    /// 1. Getting the artifacts SAS URL for the image fails
    /// 2. Listing the blobs from the Azure Storage fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use futures::StreamExt;
    /// # use freta::{Client, ImageFormat::Lime, ImageId, Result};
    /// # async fn example(client: Client, image_id: ImageId) -> Result<()> {
    /// let mut stream = client.artifacts_list(image_id);
    /// while let Some(entry) = stream.next().await {
    ///     let entry = entry?;
    ///     println!("{entry}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn artifacts_list(
        &self,
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
    /// # Errors
    ///
    /// This function will return an error in the follow cases:
    /// 1. Getting the artifacts SAS URL for the image fails
    /// 2. Getting the artifact fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use freta::{Client, Result, ImageId};
    /// # async fn example(client: Client, image_id: ImageId) -> Result<()> {
    /// let report = client.artifacts_get(image_id, "report.json").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn artifacts_get<N>(&self, image_id: ImageId, name: N) -> Result<Vec<u8>>
    where
        N: Into<String>,
    {
        let url = self.artifacts_get_sas(image_id).await?;
        let blob = blob_get(&url, name).await?;
        Ok(blob)
    }

    /// Download an artifact extracted from the image to a file
    ///
    /// # Errors
    ///
    /// This function will return an error in the follow cases:
    /// 1. Getting the artifacts SAS URL for the image fails
    /// 2. Downloading the artifact fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use freta::{Client, ImageFormat::Lime, Result, ImageId};
    /// # async fn example(client: Client, image_id: ImageId) -> Result<()> {
    /// client
    ///     .artifacts_download(image_id, "report.json", "/tmp/report.json")
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn artifacts_download<P, N>(
        &self,
        image_id: ImageId,
        name: N,
        output: P,
    ) -> Result<()>
    where
        P: AsRef<Path>,
        N: Into<String>,
    {
        let url = self.artifacts_get_sas(image_id).await?;
        container_blob_download(&url, name, output).await?;
        Ok(())
    }

    /// Monitor the ongoing state of an image until the analysis has completed.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following cases:
    /// 1. Getting the image fails
    /// 2. The image analysis state gets to `Failed` or is not recognized
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use freta::{Client, Result, ImageId};
    /// # async fn example(client: Client, image_id: ImageId) -> Result<()> {
    /// client.images_monitor(image_id).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn images_monitor(&self, image_id: ImageId) -> Result<Image> {
        let mut image = self.images_get(image_id).await?;
        if image.state == ImageState::Completed {
            return Ok(image);
        }

        // This will ensure we print the current state at the start of the loop
        let mut prev_state = ImageState::Completed;
        loop {
            if image.state != prev_state {
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
        }
        Ok(image)
    }

    /// List the configured webhooks
    ///
    /// # Errors
    ///
    /// This function will return an error in the follow cases:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to get their webhooks
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use freta::{Client, Result};
    /// # use futures::StreamExt;
    /// # async fn example(client: Client) -> Result<()> {
    /// let mut stream = client.webhooks_list();
    /// while let Some(entry) = stream.next().await {
    ///     let entry = entry?;
    ///     println!("{:?}", entry);
    /// }
    /// #   Ok(())
    /// # }
    /// ```
    pub fn webhooks_list(
        &self,
    ) -> Pin<Box<impl Stream<Item = std::result::Result<Webhook, crate::Error>> + Send + '_>> {
        let mut request = WebhooksListRequest { continuation: None };
        Box::pin(async_stream::try_stream! {
            loop {
                let result: WebhooksListResponse = self.backend.get("/api/webhooks", Some(&request)).await?;
                for webhook in result.webhooks {
                    yield webhook;
                }
                request.continuation = result.continuation;
                if request.continuation.is_none() {
                    break;
                }
            }
        })
    }

    /// Get information on a webhook
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to read the specified webhook
    pub async fn webhook_get(&self, webhook_id: WebhookId) -> Result<Webhook> {
        let res = self
            .backend
            .get(&format!("/api/webhooks/{webhook_id}"), None::<bool>)
            .await?;
        Ok(res)
    }

    /// Delete a webhook
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to delete the specified webhook
    pub async fn webhook_delete(&self, webhook_id: WebhookId) -> Result<WebhookBoolResponse> {
        let res = self
            .backend
            .delete(&format!("/api/webhooks/{webhook_id}"))
            .await?;
        Ok(res)
    }

    /// Update a webhook
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to update the specified webhook
    pub async fn webhook_update<S>(
        &self,
        webhook_id: WebhookId,
        url: Url,
        event_types: BTreeSet<WebhookEventType>,
        hmac_token: Option<S>,
    ) -> Result<Webhook>
    where
        S: Into<Secret>,
    {
        let hmac_token = hmac_token.map(std::convert::Into::into);

        let update = WebhookSubmit {
            url,
            hmac_token,
            event_types,
        };

        let res = self
            .backend
            .post(&format!("/api/webhooks/{webhook_id}"), update)
            .await?;
        Ok(res)
    }

    /// Ping a webhook
    ///
    /// This generates a synthetic event for a given webhook to test that it
    /// works as expected without having to analyze an image.
    ///
    /// Note: This service provides the raw response from the Freta service to
    /// enable the developers of webhook receivers to validate their HMAC
    /// calculation works as expected.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to update the specified webhook
    pub async fn webhook_ping(&self, webhook_id: WebhookId) -> Result<Bytes> {
        let res = self
            .backend
            .patch_raw(&format!("/api/webhooks/{webhook_id}"), None::<bool>)
            .await?;
        Ok(res)
    }

    /// Resend a webhook event
    ///
    /// This resends a specific event to the webhook.
    ///
    /// Note: If the event already pending being sent to the webhook endpoint,
    /// this will be a NOOP.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to resend the specified webhook event
    pub async fn webhook_resend(
        &self,
        webhook_id: WebhookId,
        webhook_event_id: WebhookEventId,
    ) -> Result<WebhookEvent> {
        let body = WebhookEventReplayRequest { webhook_event_id };
        let res = self
            .backend
            .post(&format!("/api/webhooks/{webhook_id}/logs"), Some(body))
            .await?;
        Ok(res)
    }

    /// Create a webhook
    ///
    /// # Errors
    ///
    /// This function will return an error in the following conditions:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to create a webhook
    pub async fn webhook_create<S>(
        &self,
        url: Url,
        event_types: BTreeSet<WebhookEventType>,
        hmac_token: Option<S>,
    ) -> Result<Webhook>
    where
        S: Into<Secret>,
    {
        let hmac_token = hmac_token.map(std::convert::Into::into);

        let update = WebhookSubmit {
            url,
            hmac_token,
            event_types,
        };

        let res = self.backend.post("/api/webhooks", update).await?;
        Ok(res)
    }

    /// List the logs for a specific webhook
    ///
    /// # Errors
    ///
    /// This function will return an error in the follow cases:
    /// 1. The connection to the Service fails
    /// 2. The user does not have permission to get their webhooks
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use freta::{Client, models::webhooks::WebhookId, Result};
    /// # use futures::StreamExt;
    /// # async fn example(client: Client, webhook_id: WebhookId) -> Result<()> {
    /// let mut stream = client.webhooks_logs(webhook_id);
    /// while let Some(entry) = stream.next().await {
    ///     let entry = entry?;
    ///     println!("{entry:?}");
    /// }
    /// #    Ok(())
    /// # }
    /// ```
    pub fn webhooks_logs(
        &self,
        webhook_id: WebhookId,
    ) -> Pin<Box<impl Stream<Item = std::result::Result<WebhookLog, crate::Error>> + Send + '_>>
    {
        let mut request = WebhookLogListRequest { continuation: None };
        Box::pin(async_stream::try_stream! {
            loop {
                let result: WebhookLogListResponse = self.backend.get(&format!("/api/webhooks/{webhook_id}/logs"), Some(&request)).await?;
                for webhook in result.webhook_events {
                    yield webhook;
                }
                request.continuation = result.continuation;
                if request.continuation.is_none() {
                    break;
                }
            }
        })
    }
}
