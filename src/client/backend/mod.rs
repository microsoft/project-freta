// Copyright (C) Microsoft Corporation. All rights reserved.

mod auth;
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

pub(crate) struct Backend {
    config: Config,
    http_client: reqwest::Client,
    auth: Auth,
}

impl Backend {
    pub async fn new() -> Result<Self> {
        let config = Config::load_or_default().await?;

        let http_client = ClientBuilder::new()
            .user_agent(format!("{}/{}", SDK_NAME, SDK_VERSION))
            .build()?;
        let auth = Auth::new(&config).await?;

        Ok(Self {
            config,
            http_client,
            auth,
        })
    }

    pub async fn logout() -> Result<()> {
        Auth::logout().await?;
        Ok(())
    }

    async fn execute_raw<Q>(
        &mut self,
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

        if let Some(token) = self.auth.get_token(&self.config).await? {
            builder = builder.bearer_auth(token.secret());
        }

        if let Some(body) = body {
            builder = builder.json(&body);
        } else {
            builder = builder.header("Content-Length", "0");
        }

        let res = builder.send().await?;

        if res.status() == reqwest::StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS {
            let body = res.bytes().await?;
            let eula = String::from_utf8_lossy(&body).to_string();
            return Err(Error::Eula(eula));
        }

        let res = res.error_for_status()?;
        let body = res.bytes().await?;
        trace!("response body: {:?}", body);
        Ok(body)
    }

    async fn execute<Q, R>(
        &mut self,
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

    pub async fn get_raw<Q>(&mut self, path: &str, query: Option<Q>) -> Result<Bytes>
    where
        Q: Serialize,
    {
        self.execute_raw(reqwest::Method::GET, path, query, None)
            .await
    }

    pub async fn get<Q, R>(&mut self, path: &str, query: Option<Q>) -> Result<R>
    where
        Q: Serialize,
        R: DeserializeOwned,
    {
        self.execute(reqwest::Method::GET, path, query, None).await
    }

    pub async fn post<Q, R>(&mut self, path: &str, body: Q) -> Result<R>
    where
        Q: Serialize,
        R: DeserializeOwned,
    {
        self.execute(reqwest::Method::POST, path, None, Some(body))
            .await
    }

    pub async fn delete<R>(&mut self, path: &str) -> Result<R>
    where
        R: DeserializeOwned,
    {
        self.execute(reqwest::Method::DELETE, path, None::<bool>, None::<bool>)
            .await
    }

    pub async fn patch<Q, R>(&mut self, path: &str, body: Q) -> Result<R>
    where
        Q: Serialize,
        R: DeserializeOwned,
    {
        self.execute(reqwest::Method::PATCH, path, None, Some(body))
            .await
    }
}
