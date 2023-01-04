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
//! ```
#![forbid(unsafe_code)]
#![deny(
    absolute_paths_not_starting_with_crate,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_link_with_quotes,
    clippy::doc_markdown,
    clippy::empty_line_after_outer_attr,
    clippy::empty_structs_with_brackets,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::float_cmp_const,
    clippy::float_equality_without_abs,
    clippy::indexing_slicing,
    clippy::manual_assert,
    clippy::missing_const_for_fn,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::option_if_let_else,
    clippy::panic,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::semicolon_if_nothing_returned,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::suspicious_operation_groupings,
    clippy::unseparated_literal_suffix,
    clippy::unused_self,
    clippy::unwrap_used,
    clippy::use_debug,
    clippy::used_underscore_binding,
    clippy::useless_let_if_seq,
    clippy::wildcard_dependencies,
    clippy::wildcard_imports,
    dead_code,
    deprecated,
    deprecated_in_future,
    exported_private_dependencies,
    future_incompatible,
    invalid_doc_attributes,
    keyword_idents,
    macro_use_extern_crate,
    missing_debug_implementations,
    missing_docs,
    non_ascii_idents,
    nonstandard_style,
    noop_method_call,
    trivial_bounds,
    trivial_casts,
    unreachable_code,
    unreachable_patterns,
    unreachable_pub,
    // unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces
)]

/// client implementation
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
