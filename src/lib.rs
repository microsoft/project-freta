// Copyright (C) Microsoft Corporation. All rights reserved.

//! Freta Client
//!
//! This crate enables communication with the [Project Freta service](https://freta.microsoft.com).
//!
//! # Example
//!
//! ```rust,no_run
//! use freta::{Client, ImageFormat::Lime, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let mut client = Client::new().await?;
//! let image = client.images_upload(Lime, [("name", "test image")], "./image.lime").await?;
//! client.images_monitor(image.image_id).await?;
//! client.artifacts_download(image.image_id, "report.json", "./report.json").await?;
//! println!("{:?}", image);
//! # Ok(())
//! # }

#![forbid(unsafe_code)]
#![deny(
    absolute_paths_not_starting_with_crate,
    dead_code,
    deprecated,
    deprecated_in_future,
    exported_private_dependencies,
    future_incompatible,
    invalid_doc_attributes,
    macro_use_extern_crate,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    trivial_bounds,
    trivial_casts,
    unreachable_code,
    unreachable_patterns,
    unreachable_pub
)]

#[cfg(feature = "client")]
mod client;

/// common data strucutures used by Freta
pub mod models;

#[cfg(feature = "client")]
pub use crate::client::{
    argparse,
    config::{ClientId, Config, Secret},
    error::{Error, Result},
    Client,
};

pub use crate::models::base::{Image, ImageFormat, ImageId, ImageState, OwnerId};

/// Name of the SDK
const SDK_NAME: &str = env!("CARGO_PKG_NAME");

/// Version of the SDK
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");
