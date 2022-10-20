// Copyright (C) Microsoft Corporation. All rights reserved.

use crate::models::base::{Image, ImageFormat, ImageId, ImageState, OwnerId};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
pub struct ImageResponse(pub Image);

#[derive(Serialize, Deserialize)]
pub struct ImageReanalyzeResponse(pub bool);

#[derive(Serialize, Deserialize)]
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
    // this should be considered an opaque field where the internal format of
    // the content can and will change in the future.
    pub continuation: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ImagesListResponse {
    pub images: Vec<Image>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageCreate {
    pub format: ImageFormat,
    pub tags: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUpdate {
    pub tags: Option<BTreeMap<String, String>>,
    pub shareable: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Info {
    pub api_version: String,
    pub models_version: String,
    pub current_eula: String,
    pub formats: Vec<ImageFormat>,
}

#[must_use]
#[inline]
pub fn bool_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserConfig {
    pub eula_accepted: Option<String>,

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

#[derive(Serialize, Deserialize)]
pub struct UserConfigUpdateResponse(pub bool);
