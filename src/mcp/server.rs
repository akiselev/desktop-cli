use crate::error::Result;
use crate::gemini::GeminiClient;
use crate::mcp::tools::{get_tool_definitions, ToolsState};
#[cfg(windows)]
use crate::mcp::tools::{
    handle_detect_elements, handle_execute_instructions, handle_list_windows,
    handle_take_screenshot,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

/// MCP server configuration
pub struct McpServerConfig {
    pub port: u16,
    pub gemini_client: GeminiClient,
    pub allowed_executables: Vec<String>,
}

/// Start the MCP server
pub async fn start_server(config: McpServerConfig) -> Result<()> {
    let state = Arc::new(ToolsState::new(
        config.gemini_client,
        config.allowed_executables,
    ));

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/tools/list", get(list_tools))
        .route("/tools/call", post(call_tool))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    tracing::info!("MCP server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| crate::error::DesktopMcpError::IoError(e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::DesktopMcpError::McpError(format!("Server error: {}", e)))?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "desktop-mcp"
    }))
}

/// List available tools
async fn list_tools() -> impl IntoResponse {
    let tools = get_tool_definitions();
    Json(serde_json::json!({
        "tools": tools
    }))
}

/// Tool call request
#[derive(Debug, Deserialize)]
struct ToolCallRequest {
    tool: String,
    parameters: Value,
}

/// Tool call response
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ToolCallResponse {
    Success { result: Value },
    Error { error: String },
}

/// Handle tool call
async fn call_tool(
    State(state): State<Arc<ToolsState>>,
    Json(request): Json<ToolCallRequest>,
) -> Response {
    tracing::info!("Tool call: {}", request.tool);

    #[cfg(windows)]
    let result = match request.tool.as_str() {
        "list_windows" => handle_list_windows(state, request.parameters).await,
        "take_screenshot" => handle_take_screenshot(state, request.parameters).await,
        "execute_instructions" => handle_execute_instructions(state, request.parameters).await,
        "detect_elements" => handle_detect_elements(state, request.parameters).await,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ToolCallResponse::Error {
                    error: format!("Unknown tool: {}", request.tool),
                }),
            )
                .into_response();
        }
    };

    #[cfg(not(windows))]
    let result: Result<serde_json::Value> = Err(crate::error::DesktopMcpError::ConfigError(
        "Windows automation tools are only available on Windows platform".to_string(),
    ));

    match result {
        Ok(value) => (
            StatusCode::OK,
            Json(ToolCallResponse::Success { result: value }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ToolCallResponse::Error {
                error: format!("{}", e),
            }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_request_deserialize() {
        let json = r#"{
            "tool": "list_windows",
            "parameters": {
                "executable_path": "notepad.exe"
            }
        }"#;

        let request: ToolCallRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.tool, "list_windows");
    }
}
