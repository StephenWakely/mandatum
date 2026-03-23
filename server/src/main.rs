mod db;
mod mcp;
mod sse;
mod tools;

use axum::{
    extract::{Path, Query, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use db::Database;
use sse::SseBroadcaster;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub broadcaster: SseBroadcaster,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("mandatum_server=info".parse().unwrap()),
        )
        .init();

    let db = Database::new("tasks.db").await.expect("Failed to open database");
    let broadcaster = SseBroadcaster::new();
    let state = Arc::new(AppState { db, broadcaster });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any);

    let rest = Router::new()
        .route("/api/tasks", get(list_tasks_handler).post(create_task_handler))
        .route("/api/tasks/:id", get(get_task_handler).patch(update_task_handler).delete(delete_task_handler))
        .route("/api/activity", get(list_activity_handler))
        .route("/api/agents", get(list_agents_handler))
        .route("/api/stats", get(stats_handler))
        .route("/events", get(sse::sse_handler))
        .layer(cors.clone())
        .with_state(state.clone());

    let mcp = mcp::create_router(state).layer(cors);

    tracing::info!("REST API  → http://0.0.0.0:3001");
    tracing::info!("MCP/SSE   → http://0.0.0.0:3002/sse");

    let rest_listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    let mcp_listener = tokio::net::TcpListener::bind("0.0.0.0:3002").await.unwrap();

    let _ = tokio::join!(
        axum::serve(rest_listener, rest),
        axum::serve(mcp_listener, mcp),
    );
}

// ── REST handlers ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TaskFilters {
    status: Option<String>,
    role: Option<String>,
    agent_id: Option<String>,
}

async fn list_tasks_handler(
    State(s): State<Arc<AppState>>,
    Query(f): Query<TaskFilters>,
) -> impl IntoResponse {
    match s.db.list_tasks(f.status, f.role, f.agent_id).await {
        Ok(tasks) => Json(tasks).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_task_handler(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match s.db.get_task_with_activity(&id).await {
        Ok(Some(t)) => Json(t).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateTaskBody {
    title: String,
    description: Option<String>,
    priority: Option<String>,
    assigned_role: Option<String>,
    tags: Option<Vec<String>>,
}

async fn create_task_handler(
    State(s): State<Arc<AppState>>,
    Json(body): Json<CreateTaskBody>,
) -> impl IntoResponse {
    match s.db.create_task(
        None,
        &body.title,
        body.description.as_deref(),
        body.priority.as_deref().unwrap_or("medium"),
        body.assigned_role.as_deref(),
        &body.tags.unwrap_or_default(),
    ).await {
        Ok(task) => {
            s.broadcaster.broadcast(
                serde_json::json!({"event":"task_created","data":task}).to_string()
            );
            (StatusCode::CREATED, Json(task)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct UpdateTaskBody {
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    assigned_role: Option<String>,
    assigned_agent_id: Option<String>,
    output_path: Option<String>,
    tags: Option<Vec<String>>,
}

async fn update_task_handler(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTaskBody>,
) -> impl IntoResponse {
    match s.db.update_task(
        &id,
        body.title.as_deref(),
        body.description.as_deref(),
        body.status.as_deref(),
        body.priority.as_deref(),
        body.assigned_role.as_deref(),
        body.assigned_agent_id.as_deref(),
        body.output_path.as_deref(),
        body.tags.as_deref(),
    ).await {
        Ok(Some(task)) => {
            s.broadcaster.broadcast(
                serde_json::json!({"event":"task_updated","data":task}).to_string()
            );
            Json(task).into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_task_handler(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match s.db.delete_task(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_activity_handler(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    match s.db.list_activity(100).await {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_agents_handler(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    match s.db.list_agents().await {
        Ok(agents) => Json(agents).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn stats_handler(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    match s.db.get_stats().await {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
