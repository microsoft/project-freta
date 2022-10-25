// Copyright (C) Microsoft Corporation. All rights reserved.

use std::borrow::Cow;

/// Freta errors
#[derive(thiserror::Error)]
pub enum Error {
    /// Authenticating to the service failed
    #[error("authentication failed: {0}")]
    Auth(&'static str),

    /// EULA error
    #[error("must agree to EULA to continue")]
    Eula(String),

    /// Data structure serialization failures
    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),

    /// IO Errors
    #[error("IO Error")]
    Io(#[from] std::io::Error),

    /// The service responded in an unexpected fashion
    #[error("invalid response: {0}")]
    InvalidResponse(&'static str),

    /// Analysis of the image failed
    #[error("analysis failed: {0}")]
    AnalysisFailed(Cow<'static, str>),

    /// Invalid OAuth2 authentication token
    #[error("invalid token: {0}")]
    InvalidToken(&'static str),

    /// Invalid SAS token
    #[error("invalid sas: {0}")]
    InvalidSas(&'static str),

    /// Unable to find the user's home directory
    #[error("unable to find $HOME")]
    MissingHome,

    /// There was an error interacting with an Azure service
    #[error("azure error")]
    Azure(#[from] azure_core::Error),

    /// HTTP error
    #[error(transparent)]
    Request(#[from] reqwest::Error),

    /// Error serializing URL parameters
    #[error(transparent)]
    UrlSerialization(#[from] serde_urlencoded::ser::Error),

    /// Error generating the status bar
    #[error(transparent)]
    StatusBar(#[from] indicatif::style::TemplateError),

    /// Data conversion errors
    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),

    /// Error converting a value into a known file extension
    #[error("file extension error: {0}")]
    Extension(Cow<'static, str>),

    /// Otherwise unspecified error
    #[error("{0}: {1}")]
    Other(&'static str, String),
}

/// Freta Result wrapper
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn format_error(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter,
) -> std::fmt::Result {
    write!(f, "error: {}", e)?;

    let mut source = e.source();

    if e.source().is_some() {
        writeln!(f, "\ncaused by:")?;
        let mut i: usize = 0;
        while let Some(inner) = source {
            writeln!(f, "{: >5}: {}", i, inner)?;
            source = inner.source();
            i += 1;
        }
    }

    Ok(())
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format_error(self, f)
    }
}
