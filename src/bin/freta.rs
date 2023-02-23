// Copyright (C) Microsoft Corporation. All rights reserved.

//! Freta Command Line Client
//!
//! This CLI implements the Freta client for use in the command line.

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
    unused_extern_crates,
    unused_import_braces
)]

use clap::{Parser, Subcommand, ValueEnum};
use cli_table::{print_stdout, Cell, CellStruct, Style, Table};
use freta::{
    argparse::parse_key_val,
    models::webhooks::{WebhookEventId, WebhookEventType, WebhookId},
    Client, ClientId, Config, Error, ImageFormat, ImageId, ImageState, OwnerId, Result, Secret,
};
use futures::{Stream, StreamExt};
use log::info;
use serde::ser::{SerializeSeq, Serializer};
use serde_json::{ser::PrettyFormatter, Value};
use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
    pin::Pin,
};
use tokio::io::{self, AsyncWriteExt};
use url::Url;

/// Third-party library license details
const LICENSES: &str = include_str!(concat!(env!("OUT_DIR"), "/licenses.json"));

/// The default fields for image list output used in `CSV` and `Table` format
const IMAGE_LIST_FIELDS: &[&str] = &["image_id", "owner_id", "state", "format"];

#[derive(Parser)]
#[clap(version, author, about = Some("Project Freta client"))]
/// Freta client
struct Args {
    #[command(subcommand)]
    /// Freta subcommands
    subcommand: SubCommands,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
/// Output formats for `list` commands
enum OutputFormat {
    /// Output in JSON format
    Json,
    /// Output in table format
    Table,
    /// Output in CSV format
    Csv,
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Csv => write!(f, "csv"),
        }
    }
}

#[derive(Subcommand)]
/// Freta subcommands
enum SubCommands {
    /// interact with the current service EULA
    Eula {
        #[clap(subcommand)]
        /// eula specific subcommands
        subcommands: EulaCommands,
    },
    /// interact with the client config
    Config {
        #[clap(subcommand)]
        /// config specific subcommands
        subcommands: ConfigCommands,
    },
    /// Login to the service
    Login,
    /// Logout of the service
    Logout,
    /// Display the license information for third-party libraries
    Licenses,
    /// Display basic information for the service
    Info,
    /// Manage images
    Images {
        #[clap(subcommand)]
        /// image specific subcommands
        subcommands: ImagesCommands,
    },
    /// Manage artifacts
    Artifacts {
        #[clap(subcommand)]
        /// Artifacts subcommands
        subcommands: ArtifactsCommands,
    },
    /// Manage webhooks
    Webhooks {
        #[clap(subcommand)]
        /// webhook specific subcommands
        subcommands: WebhooksCommands,
    },
}

#[derive(Subcommand)]
/// accept or reject the current service EULA
enum EulaCommands {
    /// get the current EULA
    Get,
    /// accept the current EULA
    Accept,
    /// reject the current EULA
    Reject,
}

#[derive(Subcommand)]
/// Artifact specific subcommands
enum ArtifactsCommands {
    /// List artifacts for an image
    List {
        /// image id
        image_id: ImageId,

        #[arg(long, default_value_t=OutputFormat::Json)]
        /// print in table mode
        output: OutputFormat,
    },
    /// Get an artifact for an image
    Get {
        /// image id
        image_id: ImageId,

        /// name of the artifact
        path: String,

        #[clap(long)]
        /// output path
        output: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
/// webhook specific subcommands
enum WebhooksCommands {
    /// Create a new webhook
    Create {
        /// webhook url
        url: Url,

        /// webhook event types to monitor
        #[clap(required = true)]
        event_types: Vec<WebhookEventType>,

        #[clap(long)]
        /// webhook hmsecret
        hmac_token: Option<Secret>,
    },
    /// Delete an existing webhook
    Delete {
        /// unique identifier for the webhook
        webhook_id: WebhookId,
    },
    /// Get an existing webhook
    Get {
        /// unique identifier for the webhook
        webhook_id: WebhookId,
    },
    /// Update an existing webhook
    Update {
        /// webhook id
        webhook_id: WebhookId,

        /// webhook url
        url: Url,

        /// webhook event types to monitor
        #[clap(required = true)]
        event_types: Vec<WebhookEventType>,

        #[clap(long)]
        /// webhook hmsecret
        hmac_token: Option<Secret>,
    },
    /// List existing webhooks
    List {
        #[arg(long, default_value_t=OutputFormat::Json)]
        /// print in table mode
        output: OutputFormat,
    },
    /// List webhook logs
    Logs {
        /// unique identifier for the webhook
        webhook_id: WebhookId,

        #[arg(long, default_value_t=OutputFormat::Json)]
        /// print in table mode
        output: OutputFormat,
    },
    /// Test an existing webhook
    Ping {
        /// unique identifier for the webhook
        webhook_id: WebhookId,
    },
    /// Resend an event to a webhook
    Resend {
        /// unique identifier for the webhook
        webhook_id: WebhookId,

        /// unique identifier for the webhook event
        webhook_event_id: WebhookEventId,
    },
}

/// Image specific subcommands
#[derive(Subcommand)]
enum ImagesCommands {
    /// get information about an image
    Get {
        /// image id
        image_id: ImageId,
    },
    /// monitor the analysis of an image
    Monitor {
        /// image id
        image_id: ImageId,
    },
    /// delete an image
    Delete {
        /// image id
        image_id: ImageId,
    },
    /// reanalyze an image
    Reanalyze {
        /// image id
        image_id: ImageId,
    },
    /// list available images
    List {
        #[arg(long)]
        /// image id
        image_id: Option<ImageId>,

        #[arg(long)]
        /// owner id
        owner_id: Option<OwnerId>,

        #[arg(long)]
        /// state
        state: Option<ImageState>,

        #[arg(long)]
        /// include sample images
        include_samples: bool,

        #[arg(long, default_value_t=OutputFormat::Json)]
        /// print in table mode
        output: OutputFormat,

        #[arg(long, action = clap::ArgAction::Append)]
        /// fields to include when using csv and table output format.  specify multiple times to include multiple fields
        fields: Option<Vec<String>>,
    },
    /// create a new image record.  note: the image must be uploaded using other tools such as azcopy.
    Create {
        /// image format
        format: ImageFormat,

        #[clap(long, value_name = "KEY=VALUE", value_parser = parse_key_val::<String, String>, action = clap::ArgAction::Append)]
        /// specify multiple times to include multiple key/value pairs
        tags: Option<Vec<(String, String)>>,
    },
    /// create an upload an image
    Upload {
        /// image path
        path: PathBuf,

        #[clap(long)]
        /// image format
        format: Option<ImageFormat>,

        #[clap(long)]
        /// monitor
        monitor: bool,

        #[clap(long)]
        /// show result (after monitoring)
        show_result: bool,

        #[clap(long, value_name = "KEY=VALUE", value_parser = parse_key_val::<String, String>, action = clap::ArgAction::Append)]
        /// specify multiple times to include multiple key/value pairs
        tags: Option<Vec<(String, String)>>,
    },
    /// update the configuration for an image
    Update {
        /// image id
        image_id: ImageId,

        #[clap(long)]
        /// images that are shared are readable to any authenticated user
        shareable: Option<bool>,

        #[clap(long, value_name = "KEY=VALUE", value_parser = parse_key_val::<String, String>, action = clap::ArgAction::Append)]
        /// specify multiple times to include multiple key/value pairs
        tags: Option<Vec<(String, String)>>,
    },
    /// Download an image to a local file.  NOTE: This is only available for successfully analyzed images.
    Download {
        /// image id
        image_id: ImageId,

        /// output path
        path: PathBuf,
    },
}

/// Config specific subcommands
#[derive(Subcommand)]
enum ConfigCommands {
    /// reset configuration to default
    Reset,
    /// get the current configuration
    Get,
    /// update the current configuration
    Update {
        #[clap(long)]
        /// azure tenant id (used when specifying a service principal)
        tenant_id: Option<String>,

        #[clap(long)]
        /// client id (Used when specifying a service principal)
        client_id: Option<String>,

        #[clap(long)]
        /// client secret (used when specifying a service principal).  Use an
        /// empty string to remove an existing client secret
        client_secret: Option<String>,

        #[clap(long)]
        /// alternate Freta instance URL
        api_url: Option<Url>,

        #[clap(long)]
        /// alternate Scope for the Azure Identity request.  Use an empty string
        /// to remove an existing scope
        scope: Option<String>,

        #[clap(long)]
        /// do not load or save cached login tokens
        ignore_login_cache: Option<bool>,
    },
}

/// implementation for config specific subcommands
async fn config(subcommands: ConfigCommands) -> Result<()> {
    let config = match subcommands {
        ConfigCommands::Reset => {
            let config = Config::new()?;
            config.save().await?;
            info!("config reset");
            config
        }
        ConfigCommands::Get => Config::load_or_default().await?,
        ConfigCommands::Update {
            tenant_id,
            client_id,
            client_secret,
            api_url,
            scope,
            ignore_login_cache,
        } => {
            let mut config = Config::load_or_default().await?;

            if let Some(tenant_id) = tenant_id {
                config.tenant_id = tenant_id;
            }

            if let Some(api_url) = api_url {
                config.api_url = api_url;
            }

            if let Some(client_id) = client_id {
                config.client_id = ClientId::new(client_id);
            }

            // if the scope is an empty string, unset the client_secret in the config
            if let Some(scope) = scope {
                if scope.is_empty() {
                    config.scope = None;
                } else {
                    config.scope = Some(scope);
                }
            }

            // if the client_secret is an empty string, unset the client_secret in the config
            if let Some(client_secret) = client_secret {
                if client_secret.is_empty() {
                    config.client_secret = None;
                } else {
                    config.client_secret = Some(Secret::new(client_secret));
                }
            }

            if let Some(ignore_login_cache) = ignore_login_cache {
                config.ignore_login_cache = ignore_login_cache;
            }

            config.save().await?;
            info!("config updated");
            config
        }
    };
    println!("{config}");

    Ok(())
}

/// Artifact specific subcommands
async fn artifacts(subcommands: ArtifactsCommands) -> Result<()> {
    let client = Client::new().await?;
    match subcommands {
        ArtifactsCommands::List { image_id, output } => {
            let stream = client.artifacts_list(image_id);
            serialize_stream(output, None, None, stream).await
        }
        ArtifactsCommands::Get {
            image_id,
            path,
            output,
        } => {
            if let Some(output) = &output {
                client.artifacts_download(image_id, path, output).await
            } else {
                let blob = client.artifacts_get(image_id, path).await?;
                io::stdout().write_all(&blob).await?;
                Ok(())
            }
        }
    }
}

/// Images specific subcommands
async fn images(subcommands: ImagesCommands) -> Result<()> {
    let client = Client::new().await?;
    match subcommands {
        ImagesCommands::Get { image_id } => client.images_get(image_id).await.map(print_data)?,
        ImagesCommands::List {
            image_id,
            owner_id,
            state,
            include_samples,
            output,
            fields,
        } => {
            let stream = client.images_list(image_id, owner_id, state, include_samples);
            let fields = fields.unwrap_or(
                IMAGE_LIST_FIELDS
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>(),
            );
            serialize_stream(output, Some(fields), Some(("{\"images\":", "}")), stream).await
        }
        ImagesCommands::Delete { image_id } => {
            client.images_delete(image_id).await.map(print_data)?
        }
        ImagesCommands::Reanalyze { image_id } => {
            client.images_reanalyze(image_id).await.map(print_data)?
        }
        ImagesCommands::Create { format, tags } => client
            .images_create(format, tags.unwrap_or_default())
            .await
            .map(print_data)?,
        ImagesCommands::Update {
            image_id,
            tags,
            shareable,
        } => client
            .images_update(image_id, tags, shareable)
            .await
            .map(print_data)?,
        ImagesCommands::Upload {
            path,
            format,
            tags,
            monitor,
            show_result,
        } => {
            let format = if let Some(format) = format {
                format
            } else if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                let ignore_case = true;
                ImageFormat::from_str(&ext_str, ignore_case)
                    .map_err(|_| Error::Extension(ext_str.into()))?
            } else {
                return Err(Error::Extension("missing file extension".into()));
            };

            let image = client
                .images_upload(format, tags.unwrap_or_default(), &path)
                .await?;
            if monitor {
                client.images_monitor(image.image_id).await?;
                if show_result {
                    let result = client.artifacts_get(image.image_id, "report.json").await?;
                    io::stdout().write_all(&result).await?;
                }
            }
            Ok(())
        }
        ImagesCommands::Download { image_id, path } => client.images_download(image_id, path).await,
        ImagesCommands::Monitor { image_id } => client.images_monitor(image_id).await,
    }
}

/// perform eula subcommands
///
/// # Errors
///
/// This returns err in the following cases:
/// 1. Getting the EULA from the service fails
/// 2. Writing the EULA to the stdout fails
/// 3. Sending the acceptance or rejection of the EULA to the service fails
async fn eula(opts: EulaCommands) -> Result<()> {
    let client = Client::new().await?;
    match opts {
        EulaCommands::Get => {
            let eula = client.eula().await?;
            io::stdout().write_all(&eula).await?;
        }
        EulaCommands::Accept => {
            let info = client.info().await?;
            let config = client.user_config_get().await?;
            client
                .user_config_update(Some(info.current_eula), config.include_samples)
                .await?;
        }
        EulaCommands::Reject => {
            let config = client.user_config_get().await?;
            client
                .user_config_update(None, config.include_samples)
                .await?;
        }
    }

    Ok(())
}

/// Request basic service information
async fn info() -> Result<()> {
    let client = Client::new().await?;
    let info = client.info().await?;
    let as_str = serde_json::to_string_pretty(&info)?;
    println!("{as_str}");

    Ok(())
}

/// Webhook specific subcommands
async fn webhooks(subcommands: WebhooksCommands) -> Result<()> {
    let client = Client::new().await?;
    match subcommands {
        WebhooksCommands::Create {
            url,
            event_types,
            hmac_token,
        } => client
            .webhook_create(url, event_types.into_iter().collect(), hmac_token)
            .await
            .map(print_data)?,
        WebhooksCommands::Delete { webhook_id } => {
            client.webhook_delete(webhook_id).await.map(print_data)?
        }
        WebhooksCommands::Get { webhook_id } => {
            client.webhook_get(webhook_id).await.map(print_data)?
        }
        WebhooksCommands::Ping { webhook_id } => {
            let result = client.webhook_ping(webhook_id).await?;
            io::stdout().write_all(&result).await?;
            Ok(())
        }
        WebhooksCommands::Update {
            webhook_id,
            url,
            event_types,
            hmac_token,
        } => client
            .webhook_update(
                webhook_id,
                url,
                event_types.into_iter().collect(),
                hmac_token,
            )
            .await
            .map(print_data)?,
        WebhooksCommands::List { output } => {
            let stream = client.webhooks_list();
            serialize_stream(output, None, Some(("{\"webhooks\":", "}")), stream).await
        }
        WebhooksCommands::Logs { webhook_id, output } => {
            let stream = client.webhooks_logs(webhook_id);
            serialize_stream(output, None, Some(("{\"webhook_events\":", "}")), stream).await
        }
        WebhooksCommands::Resend {
            webhook_id,
            webhook_event_id,
        } => client
            .webhook_resend(webhook_id, webhook_event_id)
            .await
            .map(print_data)?,
    }
}

/// Print a `Serialize`-able object as JSON to stdout
fn print_data<D>(data: D) -> Result<()>
where
    D: serde::Serialize,
{
    let json = serde_json::to_string_pretty(&data)?;
    println!("{json}");
    Ok(())
}

/// Convert a `serde_json::Value` into a `CellStruct`
///
/// This handles converting records into a `CellStruct` for use in the table
/// creation.
fn to_cell(value: &Value) -> Result<CellStruct> {
    let as_cell = match value {
        Value::String(s) => s.cell(),
        Value::Number(n) => n.to_string().cell(),
        Value::Bool(b) => b.to_string().cell(),
        Value::Null => "null".cell(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value)?.cell(),
    };
    Ok(as_cell)
}

/// Build and display a table from a stream of `Serialize`-trait objects
///
/// # Errors
///
/// 1. If the stream errors, the error is returned
/// 2. If the record cannot be serialized, the error is returned
async fn table_serialize_stream<V>(
    fields: Option<Vec<String>>,
    mut stream: Pin<Box<impl Stream<Item = std::result::Result<V, crate::Error>>>>,
) -> Result<()>
where
    V: serde::Serialize,
{
    let mut table: Vec<Vec<CellStruct>> = Vec::new();
    let mut title = vec![];
    let mut have_title = false;

    while let Some(entry) = stream.next().await {
        let entry = entry?;
        let entry = serde_json::to_value(entry)?;

        if let Some(obj) = entry.as_object() {
            let mut row = vec![];
            for (key, value) in obj {
                if !fields.as_ref().map_or(true, |y| y.contains(key)) {
                    continue;
                }
                if !have_title {
                    title.push(key.cell().bold(true));
                }
                row.push(to_cell(value)?);
            }
            have_title = true;
            table.push(row);
        } else {
            table.push(vec![to_cell(&entry)?]);
        }
    }

    let table = table.table().title(title).bold(true);

    print_stdout(table)?;

    Ok(())
}

/// Display CSV from a stream of `Serialize`-trait objects
///
/// This will write the CSV to stdout, with nested types (like Array or Object)
/// rendered as JSON strings.
///
/// # Errors
///
/// 1. If the stream errors, the error is returned
/// 2. If the record cannot be serialized, the error is returned
async fn csv_serialize_stream<V>(
    fields: Option<Vec<String>>,
    mut stream: Pin<Box<impl Stream<Item = std::result::Result<V, crate::Error>>>>,
) -> Result<()>
where
    V: serde::Serialize,
{
    let mut ser = csv::Writer::from_writer(std::io::stdout());

    let mut wrote_headers = false;
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        let mut entry = serde_json::to_value(entry)?;
        if let Some(obj) = entry.as_object_mut() {
            obj.retain(|key, _| fields.as_ref().map_or(true, |y| y.contains(key)));

            if !wrote_headers {
                let headers = obj.keys().collect::<Vec<_>>();
                ser.write_record(headers)?;
                wrote_headers = true;
            }

            let mut values = vec![];
            for (_, value) in obj.iter_mut() {
                if value.is_object() || value.is_array() {
                    *value = serde_json::Value::String(serde_json::to_string(value)?);
                }
                values.push(value);
            }
            ser.serialize(values)?;
        } else {
            ser.serialize(&entry)?;
        }
    }
    Ok(())
}

/// Display JSON from a stream of `Serialize`-trait objects
///
/// This allows iterating over results rather than buffering everything in
/// memory prior to writing the results.
///
/// # Errors
///
/// 1. If the stream errors, the error is returned
/// 2. If the record cannot be serialized, the error is returned
async fn json_serialize_stream<V>(
    wrapper: Option<(&str, &str)>,
    mut stream: Pin<Box<impl Stream<Item = std::result::Result<V, crate::Error>>>>,
) -> Result<()>
where
    V: serde::Serialize,
{
    if let Some((prefix, _)) = &wrapper {
        print!("{prefix}");
    }
    let mut ser = serde_json::Serializer::with_formatter(std::io::stdout(), PrettyFormatter::new());
    let mut serializer = ser.serialize_seq(None)?;
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        serializer.serialize_element(&entry)?;
    }
    serializer.end()?;
    if let Some((_, suffix)) = &wrapper {
        print!("{suffix}");
    }
    Ok(())
}

/// Display values from a stream of `Serialize`-trait objects
///
/// # Errors
///
/// 1. If the stream errors, the error is returned
/// 2. If the record cannot be serialized, the error is returned
async fn serialize_stream<V>(
    output: OutputFormat,
    fields: Option<Vec<String>>,
    wrapper: Option<(&str, &str)>,
    stream: Pin<Box<impl Stream<Item = std::result::Result<V, crate::Error>>>>,
) -> Result<()>
where
    V: serde::Serialize,
{
    match output {
        OutputFormat::Table => table_serialize_stream(fields, stream).await,
        OutputFormat::Csv => csv_serialize_stream(fields, stream).await,
        OutputFormat::Json => json_serialize_stream(wrapper, stream).await,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cmd = Args::parse();
    match cmd.subcommand {
        SubCommands::Config { subcommands } => {
            config(subcommands).await?;
        }
        SubCommands::Login => {
            Client::new().await?;
        }
        SubCommands::Logout => {
            Client::logout().await?;
        }
        SubCommands::Info => {
            info().await?;
        }
        SubCommands::Images { subcommands } => {
            images(subcommands).await?;
        }
        SubCommands::Artifacts { subcommands } => {
            artifacts(subcommands).await?;
        }
        SubCommands::Webhooks { subcommands } => {
            webhooks(subcommands).await?;
        }
        SubCommands::Eula { subcommands } => {
            eula(subcommands).await?;
        }
        SubCommands::Licenses => {
            println!("{LICENSES}");
        }
    };

    Ok(())
}
