// Copyright (C) Microsoft Corporation. All rights reserved.

use crate::models::base::{Image, ImageFormat, ImageId, ImageState, OwnerId};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Result for getting an image
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageResponse(pub Image);

/// Result for requesting image be reanalyzed
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageReanalyzeResponse(pub bool);

/// Result for requesting an image be deleted
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageDeleteResponse(pub bool);

#[derive(Serialize, Deserialize, Default, Debug, Parser, Clone)]
/// list images
pub struct ImageList {
    #[arg(long)]
    /// image id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_id: Option<ImageId>,

    #[arg(long)]
    /// owner id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<OwnerId>,

    #[arg(long)]
    /// state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<ImageState>,

    #[arg(long)]

    /// include sample images
    #[serde(default)]
    pub include_samples: bool,

    #[arg(skip)]
    /// continuation value used for paging.
    ///
    /// this should be considered an opaque field where the internal format of
    /// the content can and will change in the future.
    pub continuation: Option<String>,
}

/// Image List response
#[derive(Debug, Serialize, Deserialize)]
pub struct ImagesListResponse {
    /// images
    pub images: Vec<Image>,

    /// continuation value used for paging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
}

/// Image Create

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageCreate {
    /// image format
    pub format: ImageFormat,
    /// image metadata tags
    pub tags: BTreeMap<String, String>,
}

/// Image Update
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUpdate {
    /// If provided, overwrite the `tags` for the image
    pub tags: Option<BTreeMap<String, String>>,
    /// If provided, set the `shareable` value of the image
    pub shareable: Option<bool>,
}

/// Freta service information
#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    /// current API version
    pub api_version: String,
    /// current version of the modules used by the service
    pub models_version: String,
    /// checksum of the latest EULA
    pub current_eula: String,
    /// supported image formats
    pub formats: Vec<ImageFormat>,
}

#[must_use]
#[inline]
/// helper function that always returns true
///
/// This is used to provide a default value for Serde deserialization
const fn bool_true() -> bool {
    true
}

/// Freta User Configuration
#[derive(Serialize, Deserialize, Debug)]
pub struct UserConfig {
    /// latest accepted EULA
    pub eula_accepted: Option<String>,

    /// should sample images be shown in the web portal
    #[serde(default = "bool_true")]
    pub include_samples: bool,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            eula_accepted: None,
            include_samples: true,
        }
    }
}

/// Result for updating the user's configuration settings
#[derive(Debug, Serialize, Deserialize)]
pub struct UserConfigUpdateResponse(pub bool);
