// Copyright (C) Microsoft Corporation. All rights reserved.

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
