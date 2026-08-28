//! HTTP API server

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::db::Database;
use crate::lookup::EmailLookup;
use crate::models::LookupResult;

#[derive(Clone)]
pub struct AppState {
    pub lookup: Arc<EmailLookup>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .route("/v1/lookup", post(lookup_handler))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

#[derive(Debug, Deserialize)]
struct LookupRequest {
    linkedin_url: String,
}

#[derive(Debug, Serialize)]
struct LookupResponse {
    success: bool,
    result: Option<LookupResult>,
    error: Option<String>,
}

async fn lookup_handler(
    State(state): State<AppState>,
    Json(req): Json<LookupRequest>,
) -> Result<Json<LookupResponse>, (StatusCode, String)> {
    match state.lookup.lookup(&req.linkedin_url).await {
        Ok(result) => Ok(Json(LookupResponse {
            success: true,
            result: Some(result),
            error: None,
        })),
        Err(e) => Ok(Json(LookupResponse {
            success: false,
            result: None,
            error: Some(e.to_string()),
        })),
    }
}

pub async fn serve(db: Database, brightdata_token: Option<String>, port: u16) -> anyhow::Result<()> {
    let lookup = EmailLookup::new(db, brightdata_token);
    let state = AppState {
        lookup: Arc::new(lookup),
    };

    let app = create_router(state);
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Starting API server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
