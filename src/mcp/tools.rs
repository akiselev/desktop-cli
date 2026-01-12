#[cfg(windows)]
use crate::automation::windows::{capture_screenshot, list_windows, parse_hwnd, ScreenshotMethod};
#[cfg(windows)]
use crate::error::Result;
#[cfg(windows)]
use crate::executor::Executor;
use crate::gemini::client::GeminiClient;
#[cfg(windows)]
use crate::gemini::retry::RetryStrategy;
#[cfg(windows)]
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[cfg(windows)]
use std::sync::Arc;

/// Shared state for MCP tools
pub struct ToolsState {
    pub gemini_client: GeminiClient,
    pub allowed_executables: Vec<String>,
}

impl ToolsState {
    pub fn new(gemini_client: GeminiClient, allowed_executables: Vec<String>) -> Self {
        Self {
            gemini_client,
            allowed_executables,
        }
    }
}

/// Handler for list_windows tool
#[cfg(windows)]
pub async fn handle_list_windows(
    state: Arc<ToolsState>,
    params: Value,
) -> Result<Value> {
    #[derive(Deserialize)]
    struct ListWindowsParams {
        executable_path: Option<String>,
        title_pattern: Option<String>,
    }

    let params: ListWindowsParams = serde_json::from_value(params)
        .map_err(|e| crate::error::DesktopMcpError::McpError(format!("Invalid parameters: {}", e)))?;

    // If allowed_executables is configured, filter by it
    let exe_filter = if !state.allowed_executables.is_empty() {
        // Use the first allowed executable or the user-provided one if it matches
        if let Some(ref exe) = params.executable_path {
            if !state.allowed_executables.iter().any(|allowed| exe.contains(allowed)) {
                return Err(crate::error::DesktopMcpError::ConfigError(
                    format!("Executable '{}' not in allowed list", exe),
                ));
            }
            Some(exe.as_str())
        } else {
            // No filter provided, but we have a whitelist - return empty
            Some("")  // This will match nothing
        }
    } else {
        params.executable_path.as_deref()
    };

    let windows = list_windows(exe_filter, params.title_pattern.as_deref())?;

    Ok(serde_json::to_value(windows)
        .map_err(|e| crate::error::DesktopMcpError::McpError(format!("Serialization failed: {}", e)))?)
}

/// Handler for take_screenshot tool
#[cfg(windows)]
pub async fn handle_take_screenshot(
    _state: Arc<ToolsState>,
    params: Value,
) -> Result<Value> {
    #[derive(Deserialize)]
    struct TakeScreenshotParams {
        hwnd: String,
        method: Option<String>,
    }

    let params: TakeScreenshotParams = serde_json::from_value(params)
        .map_err(|e| crate::error::DesktopMcpError::McpError(format!("Invalid parameters: {}", e)))?;

    let hwnd = parse_hwnd(&params.hwnd)?;
    let method = params
        .method
        .as_ref()
        .and_then(|m| ScreenshotMethod::from_str(m))
        .unwrap_or_default();

    let screenshot = capture_screenshot(hwnd, method)?;

    #[derive(Serialize)]
    struct ScreenshotResult {
        base64_image: String,
        width: u32,
        height: u32,
        format: String,
    }

    let result = ScreenshotResult {
        base64_image: screenshot.base64_image,
        width: screenshot.width,
        height: screenshot.height,
        format: screenshot.format,
    };

    Ok(serde_json::to_value(result)
        .map_err(|e| crate::error::DesktopMcpError::McpError(format!("Serialization failed: {}", e)))?)
}

/// Handler for execute_instructions tool
#[cfg(windows)]
pub async fn handle_execute_instructions(
    state: Arc<ToolsState>,
    params: Value,
) -> Result<Value> {
    #[derive(Deserialize)]
    struct ExecuteInstructionsParams {
        hwnd: String,
        instructions: Vec<String>,
        retry_strategy: Option<String>,
    }

    let params: ExecuteInstructionsParams = serde_json::from_value(params)
        .map_err(|e| crate::error::DesktopMcpError::McpError(format!("Invalid parameters: {}", e)))?;

    let retry_strategy = params
        .retry_strategy
        .as_ref()
        .and_then(|s| RetryStrategy::from_str(s))
        .unwrap_or_default();

    let executor = Executor::new(state.gemini_client.clone());
    let summary = executor
        .execute_instructions(&params.hwnd, params.instructions, retry_strategy, true)
        .await?;

    Ok(serde_json::to_value(summary)
        .map_err(|e| crate::error::DesktopMcpError::McpError(format!("Serialization failed: {}", e)))?)
}

/// Handler for detect_elements tool
#[cfg(windows)]
pub async fn handle_detect_elements(
    state: Arc<ToolsState>,
    params: Value,
) -> Result<Value> {
    #[derive(Deserialize)]
    struct DetectElementsParams {
        hwnd: String,
        query: String,
    }

    let params: DetectElementsParams = serde_json::from_value(params)
        .map_err(|e| crate::error::DesktopMcpError::McpError(format!("Invalid parameters: {}", e)))?;

    let hwnd = parse_hwnd(&params.hwnd)?;
    let screenshot = capture_screenshot(hwnd, ScreenshotMethod::default())?;

    let result = state
        .gemini_client
        .detect_element(&params.query, &screenshot.base64_image)
        .await
        .map_err(|e| crate::error::DesktopMcpError::GeminiError(e))?;

    Ok(serde_json::to_value(result)
        .map_err(|e| crate::error::DesktopMcpError::McpError(format!("Serialization failed: {}", e)))?)
}

/// Get tool definitions for MCP protocol
pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        serde_json::json!({
            "name": "list_windows",
            "description": "List all visible windows, optionally filtered by executable path and/or title pattern",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "executable_path": {
                        "type": "string",
                        "description": "Filter by executable path (e.g., 'C:\\Program Files\\App\\app.exe')"
                    },
                    "title_pattern": {
                        "type": "string",
                        "description": "Regex pattern to match window titles (e.g., '.*Chrome.*')"
                    }
                }
            }
        }),
        serde_json::json!({
            "name": "take_screenshot",
            "description": "Take a screenshot of a specific window by HWND",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hwnd": {
                        "type": "string",
                        "description": "Window handle (from list_windows)"
                    },
                    "method": {
                        "type": "string",
                        "enum": ["bitblt", "printwindow"],
                        "description": "Screenshot method (bitblt=fast, printwindow=reliable)"
                    }
                },
                "required": ["hwnd"]
            }
        }),
        serde_json::json!({
            "name": "execute_instructions",
            "description": "Execute a list of natural language instructions on a target window",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hwnd": {
                        "type": "string",
                        "description": "Target window handle"
                    },
                    "instructions": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Natural language instructions (e.g., ['click the save button', 'type hello world'])"
                    },
                    "retry_strategy": {
                        "type": "string",
                        "enum": ["none", "basic", "advanced"],
                        "description": "Error handling strategy (default: advanced)"
                    }
                },
                "required": ["hwnd", "instructions"]
            }
        }),
        serde_json::json!({
            "name": "detect_elements",
            "description": "Detect UI elements in a screenshot using visual grounding",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hwnd": {
                        "type": "string",
                        "description": "Window handle"
                    },
                    "query": {
                        "type": "string",
                        "description": "Natural language query (e.g., 'find all buttons')"
                    }
                },
                "required": ["hwnd", "query"]
            }
        }),
    ]
}
