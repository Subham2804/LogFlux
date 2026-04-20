//! HTTP ingest: accepts log payloads via POST.

use std::net::SocketAddr;

use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::types::LogEvent;

/// Default `source` when neither the JSON entry nor `X-Log-Source` is set.
const DEFAULT_SOURCE: &str = "http";

const HDR_SOURCE: &str = "x-log-source";
const HDR_CATEGORY: &str = "x-log-category";

/// Starts the HTTP server and blocks until shutdown.
pub async fn serve(addr: SocketAddr) -> Result<(), std::io::Error> {
    let app: Router = Router::new().route("/ingest", post(ingest_logs));
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}

#[derive(Debug, Deserialize)]
struct IngestEntry {
    raw: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    category: Option<String>,
}

/// Accepts either one object or a JSON array for batched uploads.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum IngestPayload {
    Batch(Vec<IngestEntry>),
    Single(IngestEntry),
}

#[derive(Debug, Serialize)]
struct IngestAck {
    accepted: usize,
}

fn header_trimmed(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn dispatch_ingested(events: Vec<LogEvent>) {
    let _ = events;
    // TODO: hand off to pipeline (bounded channel / worker).
}

async fn ingest_logs(headers: HeaderMap, body: Bytes) -> impl IntoResponse {
    if body.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty body").into_response();
    }

    let default_source = header_trimmed(&headers, HDR_SOURCE)
        .unwrap_or_else(|| DEFAULT_SOURCE.to_string());
    let default_category = header_trimmed(&headers, HDR_CATEGORY).unwrap_or_default();

    let payload: IngestPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("invalid json: {e}"),
            )
                .into_response();
        }
    };

    let entries: Vec<IngestEntry> = match payload {
        IngestPayload::Batch(rows) => {
            if rows.is_empty() {
                return (StatusCode::BAD_REQUEST, "empty batch").into_response();
            }
            rows
        }
        IngestPayload::Single(one) => vec![one],
    };

    let received_at_ms = now_ms();
    let events: Vec<LogEvent> = entries
        .into_iter()
        .map(|entry| {
            let source = entry
                .source
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| default_source.clone());
            let category = entry
                .category
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| default_category.clone());
            LogEvent::new(entry.raw, source, category, received_at_ms)
        })
        .collect();

    let accepted = events.len();
    dispatch_ingested(events);

    (
        StatusCode::ACCEPTED,
        Json(IngestAck { accepted }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_single_and_batch() {
        let one: IngestPayload = serde_json::from_str(r#"{"raw":"a","source":"s"}"#).unwrap();
        match one {
            IngestPayload::Single(e) => assert_eq!(e.raw, "a"),
            _ => panic!("expected single"),
        }

        let batch: IngestPayload =
            serde_json::from_str(r#"[{"raw":"x"},{"raw":"y","category":"c"}]"#).unwrap();
        match batch {
            IngestPayload::Batch(v) => assert_eq!(v.len(), 2),
            _ => panic!("expected batch"),
        }
    }
}
