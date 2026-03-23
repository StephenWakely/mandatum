use axum::{
    extract::State,
    response::{sse::Event, sse::KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{convert::Infallible, sync::Arc, time::Duration};

use crate::AppState;
use crate::tools::{handle_tool_call, tool_definitions, ToolContext};

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl JsonRpcResponse {
    fn ok(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    fn err(id: Option<Value>, code: i64, message: String) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: None, error: Some(JsonRpcError { code, message }) }
    }
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(handle_post))
        .route("/message", post(handle_post))
        .route("/sse", get(mcp_sse_handler))
        .with_state(state)
}

async fn handle_post(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    Json(dispatch(req, &state).await)
}

async fn dispatch(req: JsonRpcRequest, state: &Arc<AppState>) -> JsonRpcResponse {
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::ok(id, json!({
            "protocolVersion": "2024-11-05",
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "mandatum-task-tracker", "version": "0.1.0" }
        })),
        "notifications/initialized" => JsonRpcResponse::ok(id, json!(null)),
        "ping" => JsonRpcResponse::ok(id, json!({})),
        "tools/list" => JsonRpcResponse::ok(id, json!({ "tools": tool_definitions() })),
        "tools/call" => {
            let params = req.params.unwrap_or(json!({}));
            let name = match params["name"].as_str() {
                Some(n) => n.to_string(),
                None => return JsonRpcResponse::err(id, -32602, "Missing tool name".into()),
            };
            let arguments = params["arguments"].clone();
            let ctx = ToolContext { db: state.db.clone(), broadcaster: state.broadcaster.clone(), repo_path: state.repo_path.clone(), base_branch: state.base_branch.clone() };
            match handle_tool_call(&name, arguments, &ctx).await {
                Ok(result) => JsonRpcResponse::ok(id, json!({
                    "content": [{ "type": "text", "text": result.to_string() }]
                })),
                Err(e) => JsonRpcResponse::err(id, -32603, e),
            }
        }
        _ => JsonRpcResponse::err(id, -32601, format!("Method not found: {}", req.method)),
    }
}

async fn mcp_sse_handler(
    State(_state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        yield Ok::<Event, Infallible>(
            Event::default().event("endpoint").data(json!({"endpoint":"/message"}).to_string())
        );
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            yield Ok::<Event, Infallible>(Event::default().event("ping").data("{}"));
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new().interval(Duration::from_secs(15)).text("keep-alive"),
    )
}
