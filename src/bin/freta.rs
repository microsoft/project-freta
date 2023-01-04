// Copyright (C) Microsoft Corporation. All rights reserved.

use clap::{Parser, Subcommand};
use freta::{
    argparse::parse_key_val, Client, ClientId, Config, Error, ImageFormat, ImageId, ImageState,
    OwnerId, Result, Secret,
};
use futures::StreamExt;
use log::info;
use serde::ser::{SerializeSeq, Serializer};
use std::{path::PathBuf, str::FromStr};
use tokio::io::{self, AsyncWriteExt};
use url::Url;

pub const LICENSES: &str = include_str!("../../extra/licenses.json");

#[derive(Parser)]
#[clap(version, author, about = Some("Project Freta client"))]
struct Args {
    #[command(subcommand)]
    subcommand: SubCommands,
}

#[derive(Subcommand)]
enum SubCommands {
    Eula(EulaCmd),
    Config(ConfigCmd),
    Login,
    Logout,
    Licenses,
    Info,
    Images(ImagesCmd),
    Artifacts(ArtifactsCmd),
}

#[derive(Parser)]
struct EulaCmd {
    #[clap(subcommand)]
    eula_commands: EulaCommands,
}

#[derive(Subcommand)]
// accept or reject the current service EULA
enum EulaCommands {
    // get the current EULA
    Get,

    // accept the current EULA
    Accept,

    // reject the current EULA
    Reject,
}

#[derive(Parser)]
// get artifacts from completed analysis
struct ArtifactsCmd {
    #[clap(subcommand)]
    artifacts_commands: ArtifactsCommands,
}

#[derive(Subcommand)]
enum ArtifactsCommands {
    /// List artifacts for an image
    List(SingleImage),
    Get(ArtifactsGet),
}

#[derive(Parser)]
// interact with images
struct ImagesCmd {
    #[clap(subcommand)]
    images_commands: ImagesCommands,
}

#[derive(Subcommand)]
enum ImagesCommands {
    /// get information about an image
    Get(SingleImage),
    /// monitor the analysis of an image
    Monitor(SingleImage),
    /// delete an image
    Delete(SingleImage),
    /// reanalyze an image
    Reanalyze(SingleImage),
    List(ImageListOpts),
    Create(ImagesCreateOpts),
    Upload(ImagesUploadOpts),
    Update(ImagesUpdateOpts),
}

#[derive(Parser)]
struct SingleImage {
    image_id: ImageId,
}

#[derive(Parser)]
/// create
struct ImagesCreateOpts {
    /// image format
    format: ImageFormat,

    #[clap(long, value_name = "KEY=VALUE", value_parser = parse_key_val::<String, String>, action = clap::ArgAction::Append)]
    /// specify multiple times to include multiple key/value pairs
    tags: Option<Vec<(String, String)>>,
}

#[derive(Parser, Clone)]
/// list images
pub struct ImageListOpts {
    #[arg(long)]
    /// image id
    pub image_id: Option<ImageId>,

    #[arg(long)]
    /// owner id
    pub owner_id: Option<OwnerId>,

    #[arg(long)]
    /// state
    pub state: Option<ImageState>,

    #[arg(long)]
    /// include sample images
    pub include_samples: bool,

    #[arg(skip)]
    // this should be considered an opaque field where the internal format of
    // the content can and will change in the future.
    pub continuation: Option<String>,
}

#[derive(Parser)]
/// upload image
struct ImagesUploadOpts {
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

    /// image path
    file: PathBuf,
}

#[derive(Parser)]
struct ImagesUpdateOpts {
    /// image id
    image_id: ImageId,

    #[clap(long)]
    shareable: Option<bool>,

    #[clap(long, value_name = "KEY=VALUE", value_parser = parse_key_val::<String, String>, action = clap::ArgAction::Append)]
    /// specify multiple times to include multiple key/value pairs
    tags: Option<Vec<(String, String)>>,
}

#[derive(Parser)]
struct ArtifactsGet {
    image_id: ImageId,

    path: String,

    #[clap(long)]
    /// output path
    output: Option<PathBuf>,
}

#[derive(Parser)]
/// config
struct ConfigCmd {
    #[clap(long)]
    /// reset configuration to default
    reset: bool,

    #[clap(long)]
    tenant_id: Option<String>,

    #[clap(long)]
    client_id: Option<String>,

    #[clap(long)]
    /// note: use the empty string to remove an existing client secret
    client_secret: Option<String>,

    #[clap(long)]
    api_url: Option<Url>,

    #[clap(long)]
    /// note: use the empty string to remove an exisitng scope
    scope: Option<String>,
}

async fn set_config(config_opts: ConfigCmd) -> Result<()> {
    let config = if config_opts.reset {
        Config::new()?
    } else {
        let mut config = Config::load_or_default().await?;

        if let Some(tenant_id) = config_opts.tenant_id {
            config.tenant_id = tenant_id;
        }

        if let Some(api_url) = config_opts.api_url {
            config.api_url = api_url;
        }

        if let Some(client_id) = config_opts.client_id {
            config.client_id = ClientId::new(client_id);
        }

        // if the client_secret is an empty string, unset the client_secret in the config
        if let Some(scope) = config_opts.scope {
            if scope.is_empty() {
                config.scope = None;
            } else {
                config.scope = Some(scope);
            }
        }

        // if the client_secret is an empty string, unset the client_secret in the config
        if let Some(client_secret) = config_opts.client_secret {
            if client_secret.is_empty() {
                config.client_secret = None;
            } else {
                config.client_secret = Some(Secret::new(client_secret));
            }
        }
        config
    };

    config.save().await?;

    info!("config saved: {:?}", config);
    Ok(())
}

async fn artifacts(opts: ArtifactsCmd) -> Result<()> {
    let mut client = Client::new().await?;
    match opts.artifacts_commands {
        ArtifactsCommands::List(opts) => {
            let mut stream = client.artifacts_list(opts.image_id);
            // page through results, and build a JSON output by hand.  This is a
            // bit of a hack, but this allows us to not have to coalesce all of
            // the images_list calls in memory before serializing the output.
            let mut ser = serde_json::Serializer::new(std::io::stdout());
            let mut serializer = ser.serialize_seq(None)?;
            while let Some(name) = stream.next().await {
                let name = name?;
                serializer.serialize_element(&name)?;
            }
            serializer.end()?;
        }
        ArtifactsCommands::Get(opts) => {
            if let Some(output) = &opts.output {
                client
                    .artifacts_download(opts.image_id, &opts.path, output)
                    .await?;
            } else {
                let blob = client.artifacts_get(opts.image_id, &opts.path).await?;
                io::stdout().write_all(&blob).await?;
            }
        }
    }
    Ok(())
}

fn print_data<D>(data: D) -> Result<()>
where
    D: serde::Serialize,
{
    let json = serde_json::to_string_pretty(&data)?;
    println!("{json}");
    Ok(())
}

async fn images(images_opts: ImagesCmd) -> Result<()> {
    let mut client = Client::new().await?;
    match images_opts.images_commands {
        ImagesCommands::Get(image_get) => client
            .images_get(image_get.image_id)
            .await
            .map(print_data)?,
        ImagesCommands::List(image_list) => {
            let mut stream = client.images_list(
                image_list.image_id,
                image_list.owner_id,
                image_list.state,
                image_list.include_samples,
            );

            // page through results, and build a ImagesListResponse-like output
            // by hand.  This is a bit of a hack, but this allows us to not have
            // to coalesce all of the images_list calls in memory before
            // serializing the output.
            print!("{{\"images\":");
            let mut ser = serde_json::Serializer::new(std::io::stdout());
            let mut serializer = ser.serialize_seq(None)?;
            while let Some(image) = stream.next().await {
                let image = image?;
                serializer.serialize_element(&image)?;
            }
            serializer.end()?;
            println!("}}");
            Ok(())
        }
        ImagesCommands::Delete(image_delete) => client
            .images_delete(image_delete.image_id)
            .await
            .map(print_data)?,
        ImagesCommands::Reanalyze(image_reanalyze) => client
            .images_reanalyze(image_reanalyze.image_id)
            .await
            .map(print_data)?,
        ImagesCommands::Create(create) => client
            .images_create(create.format, create.tags.unwrap_or_default())
            .await
            .map(print_data)?,
        ImagesCommands::Update(update) => client
            .images_update(update.image_id, update.tags, update.shareable)
            .await
            .map(print_data)?,
        ImagesCommands::Upload(opts) => {
            let format = if let Some(format) = opts.format {
                format
            } else if let Some(ext) = opts.file.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                ImageFormat::from_str(&ext_str).map_err(|_| Error::Extension(ext_str.into()))?
            } else {
                return Err(Error::Extension("missing file extension".into()));
            };

            let image = client
                .images_upload(format, opts.tags.unwrap_or_default(), &opts.file)
                .await?;
            if opts.monitor {
                client.images_monitor(image.image_id).await?;
                if opts.show_result {
                    let result = client.artifacts_get(image.image_id, "report.json").await?;
                    io::stdout().write_all(&result).await?;
                }
            }
            Ok(())
        }
        ImagesCommands::Monitor(opts) => client.images_monitor(opts.image_id).await,
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
    let mut client = Client::new().await?;
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

async fn info() -> Result<()> {
    let mut client = Client::new().await?;
    let info = client.info().await?;
    let as_str = serde_json::to_string_pretty(&info)?;
    println!("{as_str}");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cmd = Args::parse();
    match cmd.subcommand {
        SubCommands::Config(config_opts) => {
            set_config(config_opts).await?;
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
        SubCommands::Images(x) => {
            images(x).await?;
        }
        SubCommands::Artifacts(x) => {
            artifacts(x).await?;
        }
        SubCommands::Eula(x) => {
            eula(x.eula_commands).await?;
        }
        SubCommands::Licenses => {
            println!("{LICENSES}");
        }
    };

    Ok(())
}
