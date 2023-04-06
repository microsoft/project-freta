// Copyright (C) Microsoft Corporation. All rights reserved.

use crate::client::{
    config::{get_config_dir, ClientId, Config, Secret},
    error::{Error, Result},
};
use azure_core::{auth::AccessToken, new_http_client};
use azure_identity::{
    client_credentials_flow,
    device_code_flow::{self},
    refresh_token,
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use time::OffsetDateTime;
use tokio::fs;
use tracing::{error, warn};

/// Developers of the Freta service use this URL as a for a local instance using
/// the Azure Functions Core Tools, which does not provide authentication.  As
/// such, when using this endpoint the auth token type should be None.
const LOCAL_DEVELOPMENT_ENDPOINT: &str = "http://localhost:7071";

/// The type of authentication token
#[derive(Debug, Serialize, Deserialize, Clone)]
enum TokenType {
    /// AAD "secret" based authentication
    ClientCredentials((AccessToken, Secret)),
    /// AAD Device Code based authentication
    DeviceCode((AccessToken, AccessToken)),
    /// Token without authentication.  Used for interaction with local development endpoint
    None,
}

#[derive(Debug, Serialize, Deserialize)]
/// Authentication token for the Freta service
pub(crate) struct Auth {
    /// The Client ID of the application used for authentication
    client_id: ClientId,
    /// The type of auth token
    token: TokenType,
    /// The time at which the token expires
    expires_on: OffsetDateTime,
}

impl Auth {
    /// Create an `Auth` object
    pub(crate) async fn new(config: &Config) -> Result<Self> {
        if config.api_url.to_string() == LOCAL_DEVELOPMENT_ENDPOINT {
            return Ok(Self::new_without_auth());
        }

        if !config.ignore_login_cache {
            if let Some(entry) = Self::new_from_cache(config).await? {
                return Ok(entry);
            }
        }

        Self::new_without_cache(config).await
    }

    /// Create an `Auth` object without authentication
    fn new_without_auth() -> Self {
        Self {
            client_id: ClientId::new("development".into()),
            token: TokenType::None,
            expires_on: OffsetDateTime::now_utc() + Duration::from_secs(60 * 60 * 24 * 365),
        }
    }

    /// Create an `Auth` object, using the existing cache if possible
    async fn new_from_cache(config: &Config) -> Result<Option<Self>> {
        if let Ok(entry) = Self::from_cache().await {
            if entry.client_id == config.client_id {
                return Ok(Some(entry));
            }
            warn!("client id changed.  clearing cache");
            Self::logout().await?;
        }
        Ok(None)
    }

    /// Create an `Auth` object without using existing cache
    async fn new_without_cache(config: &Config) -> Result<Self> {
        let auth = if let Some(secret) = config.client_secret.as_ref() {
            Self::with_client_secret(config, secret).await?
        } else {
            Self::with_service(config).await?
        };

        auth.save(config).await?;
        Ok(auth)
    }

    /// Create an `Auth` object from a client secret
    async fn with_client_secret(config: &Config, client_secret: &Secret) -> Result<Self> {
        let scope = config.get_scope();
        let now = OffsetDateTime::now_utc();

        let response = client_credentials_flow::perform(
            new_http_client(),
            config.client_id.as_str(),
            client_secret.get_secret(),
            &[&scope],
            config.tenant_id.as_str(),
        )
        .await?;

        let expires_on = response
            .expires_on
            .unwrap_or_else(|| now + Duration::from_secs(response.expires_in));

        let token = TokenType::ClientCredentials((response.access_token, client_secret.clone()));

        Ok(Self {
            client_id: config.client_id.clone(),
            token,
            expires_on,
        })
    }

    #[allow(clippy::print_stderr)]
    /// Create an `Auth` object from a device code flow
    async fn with_service(config: &Config) -> Result<Self> {
        let client_id = config.client_id.clone();
        let scope = config.get_scope();

        let device_code_flow = device_code_flow::start(
            new_http_client(),
            &config.tenant_id,
            client_id.as_str(),
            &[&scope, "offline_access"],
        )
        .await?;

        eprintln!("{}", device_code_flow.message());

        let now = OffsetDateTime::now_utc();

        // poll the device code flow until we get a fresh token
        let mut stream = Box::pin(device_code_flow.stream());

        let authorization = loop {
            let response = stream
                .next()
                .await
                .ok_or(Error::Auth("device code flow failed"))?;
            if let Ok(auth) = response {
                break auth;
            }
        };

        let expires_on = now + Duration::from_secs(authorization.expires_in);

        let access_token = authorization.access_token().clone();
        let refresh_token = authorization
            .refresh_token()
            .ok_or(Error::InvalidToken("missing refresh token"))?
            .clone();

        let token = TokenType::DeviceCode((access_token, refresh_token));

        Ok(Self {
            client_id,
            token,
            expires_on,
        })
    }

    /// refresh a `DeviceCode` based access token
    async fn refresh_device_code(
        &self,
        config: &Config,
        access_token: &AccessToken,
    ) -> Result<Self> {
        let now = OffsetDateTime::now_utc();
        let client_id = config.client_id.clone();
        if self.client_id != client_id {
            return Err(Error::Auth("client_id changed unexpectedly"));
        }

        let token = refresh_token::exchange(
            new_http_client(),
            &config.tenant_id,
            client_id.as_str(),
            None,
            access_token,
        )
        .await?;

        let expires_on = now + Duration::from_secs(token.expires_in());
        let token =
            TokenType::DeviceCode((token.access_token().clone(), token.refresh_token().clone()));
        Ok(Self {
            client_id,
            token,
            expires_on,
        })
    }

    /// refresh the client access token
    pub(crate) async fn refresh_token(&mut self, config: &Config) -> Result<()> {
        match &self.token {
            TokenType::ClientCredentials((_, secret)) => {
                let token = Self::with_client_secret(config, secret).await?;
                self.token = token.token;
                self.expires_on = token.expires_on;
                self.save(config).await?;
            }
            TokenType::DeviceCode((_, refresh_token)) => {
                let token = match self.refresh_device_code(config, refresh_token).await {
                    Ok(token) => token,
                    Err(e) => {
                        error!("Unable to refresh token: {}", e);
                        Self::with_service(config).await?
                    }
                };
                self.token = token.token;
                self.expires_on = token.expires_on;
                self.save(config).await?;
            }
            TokenType::None => {}
        }
        Ok(())
    }

    /// Get the token from the cache, refreshing it if necessary.
    pub(crate) async fn get_token(&mut self, config: &Config) -> Result<Option<AccessToken>> {
        if self.expires_on < OffsetDateTime::now_utc() {
            self.refresh_token(config).await?;
        }

        match self.token {
            TokenType::ClientCredentials((ref token, _)) => Ok(Some(token.clone())),
            TokenType::DeviceCode((ref access_token, _)) => Ok(Some(access_token.clone())),
            TokenType::None => Ok(None),
        }
    }

    /// Get the on-disk path for the authentication cache
    fn get_path() -> Result<PathBuf> {
        get_config_dir().map(|p| p.join("login.cache"))
    }

    /// Save the authentication to disk.
    async fn save(&self, config: &Config) -> Result<()> {
        if !config.ignore_login_cache {
            let path = Self::get_path()?;
            let contents = serde_json::to_string_pretty(self)?;
            fs::write(&path, contents).await?;
        }
        Ok(())
    }

    /// Remove the cached authentication from disk.
    pub(crate) async fn logout() -> Result<()> {
        let path = Self::get_path()?;
        if path.exists() {
            fs::remove_file(&path).await?;
        }
        Ok(())
    }

    /// Load the cached authentication from disk.
    async fn from_cache() -> Result<Self> {
        let path = Self::get_path()?;
        let contents = fs::read_to_string(&path).await?;
        Ok(serde_json::from_str(&contents)?)
    }
}
