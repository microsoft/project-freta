use crate::{
    models::webhooks::{Webhook, WebhookEventId, WebhookEventType, WebhookLog},
    Secret,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use url::Url;

/// Web request to create or update a webhook
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookSubmit {
    /// The webhook url
    pub url: Url,

    /// If provided, the value will be used to generate an HMAC-SHA512 of the
    /// payload, which will be added to the HTTP Headers as `X-Freta-Digest`.
    pub hmac_token: Option<Secret>,

    /// The webhook events that should be included in the
    pub event_types: BTreeSet<WebhookEventType>,
}

/// Request to list webhooks
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhooksListRequest {
    /// The continuation value used for paging
    pub continuation: Option<String>,
}

/// Response to listing webhooks
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhooksListResponse {
    /// List of webhooks
    pub webhooks: Vec<Webhook>,

    /// continuation value used for paging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
}

/// Result for requesting an image be deleted
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookBoolResponse(pub bool);

/// Request to list webhook event logs for a specific webhook
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookLogListRequest {
    /// The continuation value used for paging
    pub continuation: Option<String>,
}

/// Response to listing webhook event logs
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookLogListResponse {
    /// List of webhook event
    pub webhook_events: Vec<WebhookLog>,

    /// continuation value used for paging
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
}

/// Request to replay a webhook event
#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookEventReplayRequest {
    /// Webhook Event ID
    pub webhook_event_id: WebhookEventId,
}
