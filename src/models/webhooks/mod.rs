// Copyright (C) Microsoft Corporation. All rights reserved.

/// REST API models for Webhooks
pub mod service;

use crate::{ImageId, OwnerId, Secret};
use clap::ValueEnum;
use getrandom::getrandom;
use hmac::{Hmac, Mac};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use std::{
    collections::BTreeSet,
    fmt::{Display, Error as FmtError, Formatter},
    str::FromStr,
    time::SystemTime,
};
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

/// HTTP Header used to validate HMAC-SHA512 signatures of the webhook payloads
pub const DIGEST_HEADER: &str = "x-freta-digest";

/// Unique identifier for a `Webhook`
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct WebhookId(Uuid);

impl WebhookId {
    /// Generate a new `WebhookId`
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WebhookId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for WebhookId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}", self.0)
    }
}

impl FromStr for WebhookId {
    type Err = uuid::Error;

    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(uuid_str).map(Self)
    }
}

/// Unique identifier for a `WebhookEvent` entry
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq, JsonSchema)]
pub struct WebhookEventId(Uuid);

impl WebhookEventId {
    /// Generate a new `WebhookEventId`
    #[must_use]
    pub fn new() -> Self {
        Self(new_uuid_v7())
    }
}

impl Default for WebhookEventId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for WebhookEventId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}", self.0)
    }
}

impl FromStr for WebhookEventId {
    type Err = uuid::Error;

    fn from_str(uuid_str: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(uuid_str).map(Self)
    }
}

/// Webhook Event Types
#[derive(
    Debug, Serialize, Deserialize, Clone, ValueEnum, Ord, Eq, PartialEq, PartialOrd, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum WebhookEventType {
    #[clap(skip)]
    /// Ping event, used to validate the webhook functionality
    Ping,
    /// an Image was created
    ImageCreated,
    /// an Image was deleted
    ImageDeleted,
    /// an Image was successfully analyzed
    ImageAnalysisCompleted,
    /// an Image failed to be analyzed
    ImageAnalysisFailed,
    /// an Image State was updated
    ImageStateUpdated,
}

/// Freta Webhook Event
///
/// This struct defines the structure of a webhook event sent to user's
/// configured HTTP endpoint via HTTP POST.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct WebhookEvent {
    /// Unique identifier for the event
    pub event_id: WebhookEventId,

    /// Type of the event
    pub event_type: WebhookEventType,

    /// Timestamp of when the event occurred
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    pub timestamp: OffsetDateTime,

    /// The image that triggered the event, if applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageId>,
}

impl WebhookEvent {
    /// Create a new webhook event
    #[must_use]
    pub fn new(
        event_type: WebhookEventType,
        timestamp: OffsetDateTime,
        image: Option<ImageId>,
    ) -> Self {
        Self {
            event_id: WebhookEventId::new(),
            event_type,
            timestamp,
            image,
        }
    }
}

/// Freta errors
#[derive(thiserror::Error, Debug)]
pub enum HmacError {
    /// Unable to create an HMAC from the provided token
    #[error("invalid hmac token")]
    InvalidHmacToken,

    /// HMAC structure serialization failures
    #[error("serialization error")]
    Serialization(#[from] serde_json::Error),
}

impl WebhookEvent {
    /// Generate a HMAC for the event using the provided token
    ///
    /// # Errors
    /// This could fail if the provided token is invalid or if the event cannot be serialized
    pub fn hmac_sha512(&self, hmac_token: &Secret) -> Result<String, HmacError> {
        let event_as_bytes = serde_json::to_string(&self)?.as_bytes().to_vec();
        hmac_sha512(&event_as_bytes, hmac_token)
    }
}

/// Generate a HMAC SHA512 for a slice of bytes using the provided token
///
/// # Errors
/// This could fail if the provided token is invalid
pub fn hmac_sha512(bytes: &[u8], hmac_token: &Secret) -> Result<String, HmacError> {
    let mut mac = Hmac::<Sha512>::new_from_slice(hmac_token.get_secret().as_bytes())
        .map_err(|_| HmacError::InvalidHmacToken)?;
    mac.update(bytes);
    let result = mac.finalize().into_bytes();
    let hmac_as_string = result
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(hmac_as_string)
}

/// Webhook Event State
///
/// This enum defines the current state of sending the event to the configured
/// webhook.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum WebhookEventState {
    /// The event has not been sent to the webhook
    Pending,
    /// The event has been sent to the webhook
    Success,
    /// The event has been sent to the webhook, but the webhook responded with
    /// an error
    Failure,
}

/// Webhook configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Webhook {
    /// Timestamp of the last time the webhook was updated
    #[serde(
        rename(deserialize = "Timestamp"),
        alias = "last_updated",
        skip_serializing_if = "Option::is_none",
        default,
        with = "time::serde::rfc3339::option"
    )]
    pub last_updated: Option<OffsetDateTime>,

    /// Unique identifier of the owner of the image
    #[serde(rename(deserialize = "PartitionKey"), alias = "owner_id")]
    pub owner_id: OwnerId,

    /// Unique identifier of the webhook
    #[serde(rename(deserialize = "RowKey"), alias = "webhook_id")]
    pub webhook_id: WebhookId,

    /// The webhook url
    pub url: Url,

    /// The webhook events that should be included in the
    pub event_types: BTreeSet<WebhookEventType>,

    /// If provided, the value will be used to generate an HMAC-SHA512 of the
    /// payload, which will be added to the HTTP Headers as `X-Freta-Digest`.
    pub hmac_token: Option<Secret>,
}

impl Webhook {
    /// Create a new Webhook
    #[must_use]
    pub fn new(
        owner_id: OwnerId,
        url: Url,
        event_types: BTreeSet<WebhookEventType>,
        hmac_token: Option<Secret>,
    ) -> Self {
        Self {
            last_updated: None,
            owner_id,
            webhook_id: WebhookId::new(),
            url,
            event_types,
            hmac_token,
        }
    }
}

/// A log of recent webhook events that have fired
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookLog {
    /// Timestamp of the last time the webhook was updated
    #[serde(
        rename(deserialize = "Timestamp"),
        alias = "last_updated",
        skip_serializing_if = "Option::is_none",
        default,
        with = "time::serde::rfc3339::option"
    )]
    pub last_updated: Option<OffsetDateTime>,

    /// Unique identifier of the webhook
    #[serde(rename(deserialize = "PartitionKey"), alias = "webhook_id")]
    pub webhook_id: WebhookId,

    /// Unique identifier of the event
    #[serde(rename(deserialize = "RowKey"), alias = "event_id")]
    pub event_id: WebhookEventId,

    /// The webhook event
    pub event: WebhookEvent,

    /// The webhook event state
    pub state: WebhookEventState,

    /// The webhook event response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl WebhookLog {
    /// Create a new event for a given webhook
    #[must_use]
    pub fn new(
        webhook_id: WebhookId,
        event_type: WebhookEventType,
        timestamp: OffsetDateTime,
        image_id: Option<ImageId>,
    ) -> Self {
        let event = WebhookEvent::new(event_type, timestamp, image_id);
        Self {
            last_updated: None,
            webhook_id,
            event_id: event.event_id,
            event,
            state: WebhookEventState::Pending,
            error: None,
        }
    }
}

/// Generate a UUID following the DRAFT `UUIDv7` specification
///
/// Ref: <https://datatracker.ietf.org/doc/html/draft-peabody-dispatch-new-uuid-format#name-uuid-version-7>.
///
/// Using `UUIDv7` provides for us a unique identifier that is lexicographically
/// sortable by time.
///
/// Of note, the current `UUIDv7` draft discusses monotonicity as it relates to
/// time-based sortable values.  This implementation does not handle clock
/// rolebacks or leap seconds.  In practice, this implementation of
/// lexicographical sorting should be considered a best effort.
///
/// # Panics
///
/// This function will panic if the system is unable to return the current time
/// relative to UNIX epoch or if it is unable to get 10 random bytes.
///
/// Both of these cases model the `uuid` crate's implementation.
#[allow(clippy::expect_used, clippy::cast_possible_truncation)]
fn new_uuid_v7() -> Uuid {
    let now = SystemTime::UNIX_EPOCH
        .elapsed()
        .expect("getting elapsed time since UNIX_EPOCH should not fail")
        .as_millis() as u64;
    let mut random_bytes = [0_u8; 10];
    getrandom(&mut random_bytes).expect("getting random value failed");
    fmt_uuid_v7(now, random_bytes)
}

/// Format a timestamp and random bytes following the `UUIDv7` draft specification
///
/// The implementation is directly based off the rust crate `uuid`, which has the
/// copyright & license as stated below the link to the original implementation.
/// As the Freta crate is licensed MIT, this is compatible.  Once the `uuid`
/// crate has a stable implementation of `UUIDv7` this should be removed and the
/// `uuid` crate should be used directly instead.
///
/// Ref: <https://github.com/uuid-rs/uuid/blob/60ca9af4c18e9a5131ceb43f54af308ded4ae6c0/src/timestamp.rs#L236-L255>
///
/// ```doc
/// The Uuid Project is copyright 2013-2014, The Rust Project Developers and
/// copyright 2018, The Uuid Developers.
///
/// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
/// http://www.apache.org/licenses/LICENSE-2.0> or the MIT License <LICENSE-MIT or
/// http://opensource.org/licenses/MIT>, at your option. All files in the project
/// carrying such notice may not be copied, modified, or distributed except
/// according to those terms.
/// ```
const fn fmt_uuid_v7(millis: u64, random_bytes: [u8; 10]) -> Uuid {
    // get the first 16 bits of the timestamp
    let millis_low = (millis & 0xFFFF) as u16;
    // get the next 32 bits of the timestamp
    let millis_high = ((millis >> 16) & 0xFFFF_FFFF) as u32;

    let random_and_version =
        (random_bytes[0] as u16 | ((random_bytes[1] as u16) << 8) & 0x0FFF) | (0x7 << 12);

    let mut d4 = [0; 8];

    d4[0] = (random_bytes[2] & 0x3F) | 0x80;
    d4[1] = random_bytes[3];
    d4[2] = random_bytes[4];
    d4[3] = random_bytes[5];
    d4[4] = random_bytes[6];
    d4[5] = random_bytes[7];
    d4[6] = random_bytes[8];
    d4[7] = random_bytes[9];

    // Of note, `Uuid::from_fields` handles converting the integer values to the
    // appropriate endianness.
    Uuid::from_fields(millis_high, millis_low, random_and_version, &d4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{thread::sleep, time::Duration};

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn test_uuid_v7_format() {
        let examples = vec![
            fmt_uuid_v7(0, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            fmt_uuid_v7(1_673_483_814 * 1000, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            fmt_uuid_v7(
                1_673_483_814 * 1000,
                [11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
            ),
            fmt_uuid_v7(
                1_673_483_815 * 1000,
                [11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
            ),
        ];

        insta::assert_json_snapshot!(examples);
    }

    #[test]
    /// test the lexicographical sorting of the `UUIDv7` implementation
    ///
    /// This test may fail if it happens to span across midnight after a day
    /// which contains a leap second.
    fn test_lexicographical_sorting() {
        let two_millis = Duration::from_millis(2);
        let mut uuids = vec![];

        for _ in 0..100 {
            uuids.push(new_uuid_v7().to_string());
            // sleep 2 millis between generation, as the resolution that `UUIDv7` ensures
            // lexicographical sorting is 1 millis.  sleeping 2 millis ensures the clock used by
            // `new_uuid_v7` has at least one tick between calls.
            sleep(two_millis);
        }

        let mut sorted = uuids.clone();
        sorted.sort();

        assert_eq!(
            uuids, sorted,
            "UUIDv7 should be lexicographically sorted during generation"
        );
    }

    #[test]
    fn test_hmac() -> Result<()> {
        let event = WebhookEvent {
            event_id: WebhookEventId(Uuid::from_u128(1)),
            event_type: WebhookEventType::ImageCreated,
            timestamp: OffsetDateTime::UNIX_EPOCH,
            image: Some(Uuid::from_u128(0).into()),
        };

        let hmac = event.hmac_sha512(&Secret::new("testing"))?;
        insta::assert_json_snapshot!(hmac);
        let event_as_string = serde_json::to_string(&event)?;
        insta::assert_json_snapshot!(event_as_string);

        Ok(())
    }
}
