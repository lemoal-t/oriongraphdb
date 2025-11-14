//! HTTP server for AxonGraph context compilation

use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};

use crate::{
    CompileRequest, CompileResponse, ContextEngine, HardFilters, 
    QuerySignal, SoftPreferences,
};

/// Simplified HTTP request structure
#[derive(Debug, Deserialize)]
pub struct CompileRequestHttp {
    pub intent: String,
    pub budget_tokens: usize,
    pub workstream: Option<String>,
    pub session_id: Option<String>,  // NEW: For session-aware context
    pub user_id: Option<String>,     // NEW: For memory-aware context
    pub explain: Option<bool>,
    #[serde(default)]
    pub prefer_stages: Vec<String>,  // Soft preference for stages (including memory_*)
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Compile working set handler
async fn compile_handler(
    State(engine): State<Arc<ContextEngine>>,
    Json(req): Json<CompileRequestHttp>,
) -> Result<Json<CompileResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Received compile request: intent='{}', budget={}, session_id={:?}, user_id={:?}", 
        req.intent, req.budget_tokens, req.session_id, req.user_id);
    
    // Build filters based on workstream if provided
    let hard_filters = if let Some(ref ws) = req.workstream {
        HardFilters {
            allowed_paths: vec![format!("03_workstreams/{}/", ws)],
            required_workstreams: vec![ws.clone()],
            ..Default::default()
        }
    } else {
        HardFilters::default()
    };
    
    // Build soft preferences (allow stage biasing)
    let mut soft_prefs = SoftPreferences::default();
    if !req.prefer_stages.is_empty() {
        soft_prefs.prefer_stages = req.prefer_stages.clone();
    }
    
    // Build compile request
    let compile_req = CompileRequest {
        intent: req.intent.clone(),
        budget_tokens: req.budget_tokens,
        task_id: None,
        session_id: req.session_id.clone(),     // NEW: Pass session_id
        user_id: req.user_id.clone(),           // NEW: Pass user_id
        query_signals: vec![QuerySignal::NaturalLanguage(req.intent)],
        hard_filters,
        soft_prefs,
        explain: req.explain.unwrap_or(true),
    };
    
    // Execute compilation
    match engine.compile_workingset(compile_req).await {
        Ok(response) => {
            info!("Compilation successful: {} spans, {:.1}% utilization",
                response.workingset.spans.len(),
                response.stats.token_utilization * 100.0
            );
            Ok(Json(response))
        }
        Err(e) => {
            error!("Compilation failed: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Compilation failed".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}

/// Health check handler
async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        service: "axongraph".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Create and configure the HTTP server
pub fn create_router(engine: Arc<ContextEngine>) -> Router {
    Router::new()
        .route("/health", axum::routing::get(health_handler))
        .route("/compile_workingset", post(compile_handler))
        .with_state(engine)
}

/// Run the HTTP server
pub async fn run_server(engine: Arc<ContextEngine>, port: u16) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    info!("Starting AxonGraph server on {}", addr);
    
    let app = create_router(engine);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("✓ Server listening on {}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
