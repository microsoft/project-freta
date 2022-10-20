// Copyright (C) Microsoft Corporation. All rights reserved.

use std::borrow::Cow;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("authentication failed: {0}")]
    Auth(&'static str),

    #[error("must agree to EULA to continue")]
    Eula(String),

    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),

    #[error("IO Error")]
    Io(#[from] std::io::Error),

    #[error("invalid response: {0}")]
    InvalidResponse(&'static str),

    #[error("analysis failed: {0}")]
    AnalysisFailed(Cow<'static, str>),

    #[error("invalid token: {0}")]
    InvalidToken(&'static str),

    #[error("invalid sas: {0}")]
    InvalidSas(&'static str),

    #[error("unable to find $HOME")]
    MissingHome,

    #[error("azure error")]
    Azure(#[from] azure_core::Error),

    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    UrlSerialization(#[from] serde_urlencoded::ser::Error),

    #[error(transparent)]
    StatusBar(#[from] indicatif::style::TemplateError),

    #[error(transparent)]
    TryFromIntError(#[from] std::num::TryFromIntError),

    #[error("file extension error: {0}")]
    Extension(Cow<'static, str>),
}
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
