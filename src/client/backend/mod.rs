// Copyright (C) Microsoft Corporation. All rights reserved.

/// backend client authentication implementation
mod auth;
/// helpers for dealing with Azure Blob Storage
pub(crate) mod azure_blobs;

use crate::{
    client::{
        backend::auth::Auth,
        config::Config,
        error::{Error, Result},
    },
    SDK_NAME, SDK_VERSION,
};
use bytes::Bytes;
use log::trace;
use reqwest::ClientBuilder;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::Mutex;

#[derive(Debug)]
/// REST API client implementation
pub(crate) struct Backend {
    /// CLI configuration
    config: Config,
    /// http client
    http_client: reqwest::Client,
    /// backend authentication information
    auth: Mutex<Auth>,
}

impl Backend {
    /// Create a new backend client
    pub(crate) async fn new(config: Config) -> Result<Self> {
        let http_client = ClientBuilder::new()
            .user_agent(format!("{SDK_NAME}/{SDK_VERSION}"))
            .build()?;
        let auth = Mutex::new(Auth::new(&config).await?);

        Ok(Self {
            config,
            http_client,
            auth,
        })
    }

    /// log out of the backend
    pub(crate) async fn logout() -> Result<()> {
        Auth::logout().await?;
        Ok(())
    }

    /// send the request to the backend and return the results in `Bytes`
    async fn execute_raw<Q>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: Option<Q>,
        body: Option<Q>,
    ) -> Result<Bytes>
    where
        Q: Serialize,
    {
        let mut url = self.config.api_url.clone();
        url.set_path(path);

        if let Some(query) = query {
            let query_string = serde_urlencoded::to_string(&query)?;
            if !query_string.is_empty() {
                trace!("setting query: {}", query_string);
                url.set_query(Some(&query_string));
            }
        }

        let mut builder = self.http_client.clone().request(method, url);

        // lock self.auth while getting an auth token
        let token = {
            let mut auth = self.auth.lock().await;
            auth.get_token(&self.config).await?
        };
        if let Some(token) = token {
            builder = builder.bearer_auth(token.secret());
        }

        if let Some(json_body) = body {
            builder = builder.json(&json_body);
        } else {
            builder = builder.header("Content-Length", "0");
        }

        let res = builder.send().await?;

        if res.status() == reqwest::StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS {
            let response_body = res.bytes().await?;
            let eula = String::from_utf8_lossy(&response_body).to_string();
            return Err(Error::Eula(eula));
        }

        let res = res.error_for_status()?;
        let response_body = res.bytes().await?;
        trace!("response body: {:?}", response_body);
        Ok(response_body)
    }

    /// send the request to the backend and deserialize the response as JSON
    async fn execute<Q, R>(
        &self,
        method: reqwest::Method,
        path: &str,
        query: Option<Q>,
        body: Option<Q>,
    ) -> Result<R>
    where
        Q: Serialize,
        R: DeserializeOwned,
    {
        let body = self.execute_raw(method, path, query, body).await?;
        let as_json = serde_json::from_slice(&body)?;
        Ok(as_json)
    }

    /// Send a GET request to the backend, but return the results in `Bytes`
    pub(crate) async fn get_raw<Q>(&self, path: &str, query: Option<Q>) -> Result<Bytes>
    where
        Q: Serialize,
    {
        self.execute_raw(reqwest::Method::GET, path, query, None)
            .await
    }

    /// Send a GET request to the backend
    pub(crate) async fn get<Q, R>(&self, path: &str, query: Option<Q>) -> Result<R>
    where
        Q: Serialize,
        R: DeserializeOwned,
    {
        self.execute(reqwest::Method::GET, path, query, None).await
    }

    /// Send a PATCH request to the backend but do not deserialize the response.
    pub(crate) async fn patch_raw<Q>(&self, path: &str, body: Q) -> Result<Bytes>
    where
        Q: Serialize,
    {
        self.execute_raw(reqwest::Method::PATCH, path, None, Some(body))
            .await
    }

    /// Send a POST request to the backend.
    pub(crate) async fn post<Q, R>(&self, path: &str, body: Q) -> Result<R>
    where
        Q: Serialize,
        R: DeserializeOwned,
    {
        self.execute(reqwest::Method::POST, path, None, Some(body))
            .await
    }

    /// Send a DELETE request to the backend.
    pub(crate) async fn delete<R>(&self, path: &str) -> Result<R>
    where
        R: DeserializeOwned,
    {
        self.execute(reqwest::Method::DELETE, path, None::<bool>, None::<bool>)
            .await
    }

    /// Send a PATCH request to the backend.
    pub(crate) async fn patch<Q, R>(&self, path: &str, body: Q) -> Result<R>
    where
        Q: Serialize,
        R: DeserializeOwned,
    {
        self.execute(reqwest::Method::PATCH, path, None, Some(body))
            .await
    }
}
