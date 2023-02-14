// Copyright (C) Microsoft Corporation. All rights reserved.

use crate::{client::backend::Backend, Error, Result};
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    path::PathBuf,
};
use tokio::fs;
use url::Url;

/// Value that is printed upon trying to show a debug version of a `Secret`
const REDACTED: &str = "[redacted secret]";

#[derive(Serialize, Deserialize, Clone)]
/// Client Secret
///
/// This is an opaque type that makes it such that secrets are not accidentally
/// logged.
pub struct Secret(String);

impl Secret {
    #[must_use]
    /// Create a new `Secret`
    pub fn new<S>(secret: S) -> Self
    where
        S: Into<String>,
    {
        Self(secret.into())
    }

    /// Unwrap the secret for use.
    ///
    /// Requiring the use of `get_secret` requires being intentional about using
    /// the secret.
    pub(crate) fn get_secret(&self) -> &str {
        self.0.as_ref()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{REDACTED}")
    }
}

impl From<String> for Secret {
    fn from(secret: String) -> Self {
        Self::new(secret)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
/// AAD App client id
pub struct ClientId(String);

impl ClientId {
    #[must_use]
    /// Create a new `ClientId`
    pub const fn new(secret: String) -> Self {
        Self(secret)
    }

    /// Returns the client id as a str
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Serialize, Deserialize)]
/// Freta client Config
pub struct Config {
    /// URL for the Freta API.
    ///
    /// NOTE: For the public Freta service, this should always be `https://freta.microsoft.com`
    pub api_url: Url,

    /// AAD app registration client id
    pub client_id: ClientId,

    /// Tenant of the AAD app registration for the client
    pub tenant_id: String,

    /// Client Secrt for custom app registrations to connect to Freta
    pub client_secret: Option<Secret>,

    /// AAD App registration scope
    pub scope: Option<String>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Config");
        d.field("api url", &self.api_url.as_str());
        d.field("client id", &self.client_id.as_str());
        d.field("tenant id", &self.tenant_id.as_str());

        if self.client_secret.is_some() {
            d.field("client secret", &REDACTED);
        }

        if let Some(scope) = &self.scope {
            d.field("scope", &scope);
        }

        d.finish()
    }
}

/// Implement `Display` for the Config as `Debug` for now
impl Display for Config {
    #[allow(clippy::use_debug)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Config {
    /// Create a new `Config` with the default values
    ///
    /// # Errors
    ///
    /// This will return an error if the default API URL cannot be parsed.
    pub fn new() -> Result<Self> {
        Ok(Self {
            api_url: Url::parse("https://freta.microsoft.com/")
                .map_err(|e| Error::Other("unable to parse URL", format!("{e}")))?,
            client_id: ClientId::new("574efb07-14a8-4232-a200-89714a0324c9".into()),
            tenant_id: "common".into(),
            client_secret: None,
            scope: Some("api://a934fc14-92d7-4127-aecd-bddab35935da/.default".into()),
        })
    }

    /// Get the path for the config file
    fn get_path() -> Result<PathBuf> {
        Ok(get_config_dir()?.join("cli.config"))
    }

    /// Load the user's current configuration or use the default if that does
    /// not exist
    ///
    /// # Errors
    /// This will return an error in the following cases:
    /// 1. The path loading the configuration file cannot be determined
    /// 2. Loading the configuration file fails
    /// 3. Saving the default configuration file fails if there is not an existing file
    pub async fn load_or_default() -> Result<Self> {
        if Self::get_path()?.exists() {
            Self::load().await
        } else {
            let config = Self::new()?;
            config.save().await?;
            Ok(config)
        }
    }

    /// Returns the user's configuration from `~/.config/freta/cli.config`
    ///
    /// # Errors
    /// This will return an error in the following cases:
    /// 1. The path loading the configuration file cannot be determined
    /// 2. The configuration file cannot be read
    /// 3. The configuration file cannot be deserialized
    async fn load() -> Result<Self> {
        let path = Self::get_path()?;
        let contents = fs::read_to_string(&path).await?;
        Ok(serde_json::from_str(&contents)?)
    }

    /// Create the config directory
    ///
    /// # Errors
    /// This will return an error in the following cases:
    /// 1. The path loading the configuration file cannot be determined
    /// 2. The directory for the configuration file cannot be created
    async fn create_config_dir() -> Result<()> {
        let path = get_config_dir()?;
        fs::create_dir_all(&path).await?;
        Ok(())
    }

    /// Save the user's configuration to `~/.config/freta/cli.config`
    ///
    /// At the moment, client configuration only includes login configuration
    /// information.  Therefore, on any change, log the user out and log them
    /// back in.
    ///
    /// # Errors
    /// This will return an error if the configuration file cannot be saved
    pub async fn save(&self) -> Result<()> {
        Self::create_config_dir().await?;
        let path = Self::get_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents).await?;
        Backend::logout().await?;
        Ok(())
    }

    /// Get the JWT token scope for the current configuration
    pub(crate) fn get_scope(&self) -> String {
        self.scope.as_ref().map_or_else(
            || {
                let mut scope = self.api_url.clone();
                scope.set_path(".default");
                scope.to_string().replacen("https://", "api://", 1)
            },
            std::clone::Clone::clone,
        )
    }
}

/// return expaneded version of `$HOME/.config/freta/`
///
/// # Errors
/// This will return an error if the user's home directory cannot be determined
pub(crate) fn get_config_dir() -> Result<PathBuf> {
    home_dir()
        .ok_or(Error::MissingHome)
        .map(|x| x.join(".config/freta/"))
}
