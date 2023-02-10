// Copyright (C) Microsoft Corporation. All rights reserved.

/// An example receiver for Freta webhook events
///
/// This example shows how to receive webhook events from Freta, and depending
/// on the event type, downloading the report from the image and extract
/// specific information from the report.
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use clap::Parser;
use freta::{
    models::webhooks::{hmac_sha512, WebhookEvent, WebhookEventType, DIGEST_HEADER},
    Client, Error, ImageId, Result, Secret,
};
use log::{error, info};
use serde_json::Value;
use std::{net::SocketAddr, string::ToString};

const API_ENDPOINT: &str = "/api/freta-analysis-webhook";

#[derive(Parser)]
struct Config {
    /// Port to run the service on
    #[arg(long, default_value_t = 3000, env = "FUNCTIONS_CUSTOMHANDLER_PORT")]
    port: u16,

    #[arg(long, env = "FRETA_HMAC_TOKEN")]
    hmac_token: Option<Secret>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = Config::parse();

    let app = Router::new()
        .route(API_ENDPOINT, post(webhook_receiver))
        .with_state(config.hmac_token);
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("starting service on {}", addr);

    let service = app.into_make_service();

    axum::Server::bind(&addr)
        .serve(service)
        .await
        .map_err(|e| Error::Other("service failed", format!("{e:?}")))?;

    Ok(())
}

/// Deserialize & validate the HMAC for the webhook
fn parse_and_validate(
    bytes: &[u8],
    hmac_header: Option<String>,
    hmac_token: Option<Secret>,
) -> std::result::Result<WebhookEvent, Box<dyn std::error::Error>> {
    let event: WebhookEvent = serde_json::from_slice(bytes)?;

    // Note: `WebhookEvent.hmac_sha512` will reserialize and then hmac the
    // event.  This validates the raw bytes that came from the webhook body
    if let Some(token) = hmac_token {
        let Some(from_header) = hmac_header else {
            return Err("hmac header is required".into());
        };

        let hmac = hmac_sha512(bytes, &token)?;
        if !compare(&from_header, &hmac) {
            return Err("hmac does not match".into());
        }
    }

    Ok(event)
}

/// Comparison in constant time.
fn compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0;

    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

/// retrieve the report for an image and log the extracted kernel banner
async fn show_kernel_banner_from_report(image_id: ImageId) -> Result<()> {
    let mut client = Client::new().await?;
    let report = client.artifacts_get(image_id, "report.json").await?;
    let report_decoded: Value = serde_json::from_slice(&report)?;
    info!(
        "report: image_id:{image_id} banner:{:?}",
        report_decoded["info"]["banner"]
    );
    Ok(())
}

/// Webhook endpoint that handles receiving the webhook from Freta
///
/// # Inputs
/// * `hmac_token` - Optional HMAC token to validate the webhook payload
///    This is set by the command line arguments
/// * `headers` - HTTP Headers from the request, this is used to pull out the HMAC digest
/// * `body` - HTTP Body.  Note, this uses the raw request instead deserializing
///    in the middleware because we need to verify the HMAC digest prior to
///    deserialization
async fn webhook_receiver(
    State(hmac_token): State<Option<Secret>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // get the digest header, treating parsing errors as if the digest does not
    // exist
    let hmac_header = headers
        .get(DIGEST_HEADER)
        .and_then(|h| h.to_str().map(ToString::to_string).ok());

    let event = match parse_and_validate(&body, hmac_header, hmac_token) {
        Ok(e) => e,
        Err(err) => {
            error!("unable to parse webhook payload: {err:?}");
            return (StatusCode::BAD_REQUEST, "invalid payload");
        }
    };

    info!("decoded {event:?}");

    // This is a an example as to how to respond to events for a given image.
    if event.event_type == WebhookEventType::ImageAnalysisCompleted {
        if let Some(image_id) = event.image {
            if let Err(err) = show_kernel_banner_from_report(image_id).await {
                error!("unable to retrieve report from image: {err:?}");
            }
        }
    }

    (StatusCode::OK, "thanks")
}
