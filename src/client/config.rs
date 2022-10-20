// Copyright (C) Microsoft Corporation. All rights reserved.

use crate::{Error, Result};
use home::home_dir;
use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};
use tokio::fs;
use url::Url;

const REDACTED: &str = "[redacted]";

#[derive(Serialize, Deserialize, Clone)]
pub struct Secret(String);

impl Secret {
    #[must_use]
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    pub(crate) fn get_secret(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ClientId(String);

impl ClientId {
    #[must_use]
    pub fn new(secret: String) -> Self {
        Self(secret)
    }
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub api_url: Url,
    pub client_id: ClientId,
    pub tenant_id: String,
    pub client_secret: Option<Secret>,
    pub scope: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: Url::parse("https://freta.microsoft.com/").expect("default URL failed"),
            client_id: ClientId::new("574efb07-14a8-4232-a200-89714a0324c9".into()),
            tenant_id: "common".into(),
            client_secret: None,
            scope: Some("api://a934fc14-92d7-4127-aecd-bddab35935da/.default".into()),
        }
    }
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

impl Config {
    fn get_path() -> Result<PathBuf> {
        Ok(get_config_dir()?.join("cli.config"))
    }

    pub async fn load_or_default() -> Result<Self> {
        if Self::get_path()?.exists() {
            Self::load().await
        } else {
            let config = Self::default();
            config.save().await?;
            Ok(config)
        }
    }

    async fn load() -> Result<Self> {
        let path = Self::get_path()?;
        let contents = fs::read_to_string(&path).await?;
        Ok(serde_json::from_str(&contents)?)
    }

    async fn create_config_dir() -> Result<()> {
        let path = get_config_dir()?;
        fs::create_dir_all(&path).await?;
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        Self::create_config_dir().await?;
        let path = Self::get_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents).await?;
        Ok(())
    }

    pub(crate) fn get_scope(&self) -> String {
        if let Some(scope) = &self.scope {
            scope.clone()
        } else {
            let mut scope = self.api_url.clone();
            scope.set_path(".default");
            scope.to_string().replacen("https://", "api://", 1)
        }
    }
}

pub fn get_config_dir() -> Result<PathBuf> {
    home_dir()
        .ok_or(Error::MissingHome)
        .map(|x| x.join(".config/freta/"))
}
