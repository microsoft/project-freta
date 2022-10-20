// Copyright (C) Microsoft Corporation. All rights reserved.

//! Freta Client
//!
//! This crate enables communication with the [Project Freta service](https://freta.microsoft.com).
//!
//! # Example
//!
//! ```no_run
//! use freta::{Client, ImageFormat::Lime, Result};
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! let mut client = Client::new().await?;
//! let image = client.images_upload(Lime, [("name", "test image")], "./image.lime").await?;
//! println!("{:?}", image);
//! # Ok(())
//! # }
//!

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations, nonstandard_style, future_incompatible)]
// #![warn(missing_docs, unreachable_pub)]

#[cfg(feature = "client")]
mod client;
pub mod models;

#[cfg(feature = "client")]
pub use crate::client::{
    argparse,
    config::{ClientId, Config, Secret},
    error::{Error, Result},
    Client,
};

pub use crate::models::base::{Image, ImageFormat, ImageId, ImageState, OwnerId};

pub const SDK_NAME: &str = env!("CARGO_PKG_NAME");
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");
