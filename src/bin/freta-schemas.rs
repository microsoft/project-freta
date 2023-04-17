// Copyright (C) Microsoft Corporation. All rights reserved.

use assert_json_diff::{assert_json_matches_no_panic, CompareMode, Config as DiffConfig};
use clap::{Parser, ValueEnum};
use freta::{models::webhooks::WebhookEvent, Error, Result};
use schemars::{schema::RootSchema, schema_for};
use std::{fs::OpenOptions, path::PathBuf};

/// schema to generate
///
/// For now, this only includes the webhook schema.  However, future schemas
/// will be added here.
#[derive(Debug, Eq, PartialEq, Clone, ValueEnum)]
pub enum SchemaType {
    /// Freta Webhook event schema
    WebhookEvent,
}

#[derive(Parser)]
/// Generate a JSON Schema for Freta
struct Config {
    /// schema to generate
    schema: SchemaType,

    /// file to analyze
    file: PathBuf,

    /// check against existing schema
    #[arg(long)]
    check: bool,
}

fn get_existing(path: &PathBuf) -> Result<RootSchema> {
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| Error::Io {
            message: format!("reading schema: {path:?}").into(),
            source: e,
        })?;
    let result = serde_json::from_reader(file)?;
    Ok(result)
}

fn write_schema(schema: &RootSchema, path: &PathBuf) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|e| Error::Io {
            message: format!("writing schema: {path:?}").into(),
            source: e,
        })?;
    serde_json::to_writer_pretty(file, &schema)?;
    Ok(())
}

fn main() -> Result<()> {
    let config = Config::parse();

    let current = match config.schema {
        SchemaType::WebhookEvent => schema_for!(WebhookEvent),
    };

    if config.check {
        let existing_json = get_existing(&config.file)?;
        assert_json_matches_no_panic(
            &current,
            &existing_json,
            DiffConfig::new(CompareMode::Strict),
        )
        .map_err(|e| Error::Other("schemas differ", e))?;
    } else {
        write_schema(&current, &config.file)?;
    }

    Ok(())
}
