// Copyright (C) Microsoft Corporation. All rights reserved.

use clap::ValueEnum;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::BTreeMap,
    fmt::{Display, Error as FmtError, Formatter},
    str::FromStr,
};
use strum_macros::EnumIter;
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub struct ImageId(Uuid);

impl ImageId {
    #[must_use]
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ImageId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ImageId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ImageId {
    type Err = uuid::Error;

    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(uuid_str).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerId {
    pub tenant_id: Uuid,
    pub oid: Uuid,
}

impl OwnerId {
    #[must_use]
    pub const fn samples() -> Self {
        Self {
            tenant_id: Uuid::from_u128(0),
            oid: Uuid::from_u128(0),
        }
    }
}

impl Display for OwnerId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}_{}", self.tenant_id, self.oid)
    }
}

impl Serialize for OwnerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&format!("{}", self))
    }
}

impl FromStr for OwnerId {
    type Err = Box<dyn std::error::Error + Send + Sync + 'static>;

    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        match uuid_str.split_once('_') {
            Some((tenant_id, oid)) => Ok(Self {
                tenant_id: Uuid::parse_str(tenant_id)?,
                oid: Uuid::parse_str(oid)?,
            }),
            None => Err("invalid owner_id".into()),
        }
    }
}

impl<'de> serde::Deserialize<'de> for OwnerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, ValueEnum, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageState {
    WaitingForUpload,
    ToQueue,
    Queued,
    Running,
    Finalizing,
    Completed,
    Failed,
    Deleting,
}

impl ImageState {
    #[must_use]
    pub fn can_reimage(&self) -> bool {
        match self {
            ImageState::WaitingForUpload
            | ImageState::Running
            | ImageState::Deleting
            | ImageState::ToQueue
            | ImageState::Queued => false,
            ImageState::Failed | ImageState::Completed | ImageState::Finalizing => true,
        }
    }

    #[must_use]
    pub fn can_reimage_states() -> Vec<Self> {
        let mut results = vec![];
        for variant in Self::value_variants() {
            if variant.can_reimage() {
                results.push(variant.clone());
            }
        }
        results
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, EnumIter, ValueEnum, Clone, Eq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Vmrs,
    Raw,
    Lime,
    Core,
    Avmh,
}

#[derive(Debug)]
pub struct ParseError {}

impl FromStr for ImageFormat {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x = match s {
            "vmrs" => Self::Vmrs,
            "raw" => Self::Raw,
            "lime" => Self::Lime,
            "core" => Self::Core,
            "avmh" => Self::Avmh,
            _ => return Err(ParseError {}),
        };
        Ok(x)
    }
}

impl Display for ImageFormat {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Image {
    #[serde(
        rename(deserialize = "Timestamp"),
        alias = "last_updated",
        skip_serializing_if = "Option::is_none",
        default,
        with = "time::serde::rfc3339::option"
    )]
    pub last_updated: Option<OffsetDateTime>,
    #[serde(rename(deserialize = "PartitionKey"), alias = "owner_id")]
    pub owner_id: OwnerId,
    #[serde(rename(deserialize = "RowKey"), alias = "image_id")]
    pub image_id: ImageId,
    pub state: ImageState,
    pub format: ImageFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<Url>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts_url: Option<Url>,
    #[serde(default = "BTreeMap::new")]
    pub tags: BTreeMap<String, String>,
    #[serde(default)]
    pub shareable: bool,
}

impl Image {
    #[must_use]
    pub fn new(owner_id: OwnerId, format: ImageFormat, tags: BTreeMap<String, String>) -> Self {
        Self {
            last_updated: None,
            owner_id,
            image_id: ImageId::new(),
            state: ImageState::WaitingForUpload,
            format,
            error: None,
            image_url: None,
            artifacts_url: None,
            tags,
            shareable: false,
        }
    }
}
