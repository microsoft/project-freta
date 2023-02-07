// Copyright (C) Microsoft Corporation. All rights reserved.

use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::BTreeMap,
    fmt::{Display, Error as FmtError, Formatter},
    str::FromStr,
};
use strum_macros::{Display, EnumIter};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

/// Unique identifier for an `Image`
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, JsonSchema)]
pub struct ImageId(Uuid);

impl ImageId {
    /// Generate a new `ImageId`
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

impl From<Uuid> for ImageId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The owner of an image
pub struct OwnerId {
    /// The AAD tenant of the owner
    pub tenant_id: Uuid,
    /// The AAD `oid` of the user
    pub oid: Uuid,
}

impl OwnerId {
    /// The `OwnerId` associated with sample images
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
        serializer.collect_str(&format!("{self}"))
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

/// State of an Image
#[derive(Display, Debug, Serialize, Deserialize, PartialEq, Clone, ValueEnum, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageState {
    /// The service has not received notification the upload has completed
    WaitingForUpload,
    /// The image is ready to be queued
    ToQueue,
    /// The image has been queued for analysis
    Queued,
    /// The image is currently being analyzed
    Running,
    /// The results of the analysis are being uploaded
    Finalizing,
    /// The analysis has completed successfully
    Completed,
    /// The analysis of the image failed
    Failed,
    /// The image and it's related artifacts are currently being deleted
    Deleting,
}

impl ImageState {
    /// Is the image state such that re-analyzing is possible
    #[must_use]
    pub const fn can_reimage(&self) -> bool {
        match self {
            ImageState::WaitingForUpload
            | ImageState::Running
            | ImageState::Deleting
            | ImageState::ToQueue
            | ImageState::Queued => false,
            ImageState::Failed | ImageState::Completed | ImageState::Finalizing => true,
        }
    }

    /// Return the set of states that where re-analyzing is possible
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

/// Format for an Image
#[derive(Debug, Serialize, Deserialize, PartialEq, EnumIter, ValueEnum, Clone, Eq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    /// Hyper-V 'checkpoint' files
    Vmrs,
    /// RAW memory dumps, such as created with `dd`
    Raw,
    /// Lime memory dumps, as created with AVML or LiME
    Lime,
    /// Full-system Linux core dumps, such as memory dumps as created by VirtualBox or Dumpit for Linux
    Core,
    /// Internal memory snapshot feature
    Avmh,
}

/// Error converting a string into an `ImageFormat`
#[derive(Debug)]
pub struct ParseError;
impl std::error::Error for ParseError {}
impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        write!(f, "parsing error")
    }
}

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
        match self {
            Self::Vmrs => write!(f, "vmrs"),
            Self::Raw => write!(f, "raw"),
            Self::Lime => write!(f, "lime"),
            Self::Core => write!(f, "core"),
            Self::Avmh => write!(f, "avmh"),
        }
    }
}

/// Image entry in the Freta service
#[derive(Serialize, Deserialize, Debug)]
pub struct Image {
    /// Timestamp of the last time the image entry was updated
    #[serde(
        rename(deserialize = "Timestamp"),
        alias = "last_updated",
        skip_serializing_if = "Option::is_none",
        default,
        with = "time::serde::rfc3339::option"
    )]
    pub last_updated: Option<OffsetDateTime>,

    /// Unique identifier of the owner of the image
    #[serde(rename(deserialize = "PartitionKey"), alias = "owner_id")]
    pub owner_id: OwnerId,

    /// Unique identifier of the Image
    #[serde(rename(deserialize = "RowKey"), alias = "image_id")]
    pub image_id: ImageId,

    /// Current state of the image
    pub state: ImageState,

    /// Format of the image
    pub format: ImageFormat,

    /// Error of the last analysis
    ///
    /// NOTE: This is only provided if the analysis previously failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// SAS URL for downloading the image snapshot.
    ///
    /// NOTE: This is only provided for successfully analyzed images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<Url>,

    /// SAS URL for downloading the artifacts of an image.
    ///
    /// NOTE: This is only provided for successfully analyzed images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts_url: Option<Url>,

    /// Key-Value pair of metadata associated with the image
    #[serde(default = "BTreeMap::new")]
    pub tags: BTreeMap<String, String>,

    /// Is the image accessible by authenticated users that know the ImageId
    #[serde(default)]
    pub shareable: bool,
}

impl Image {
    /// Create a new Image
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
