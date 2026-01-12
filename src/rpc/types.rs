use remoc::rtc::CallError;
use serde::{Deserialize, Serialize};

/// Error type for RPC service calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceError {
    /// Remote call failed
    Call(String),
    /// Window not found
    WindowNotFound(String),
    /// Screenshot capture failed
    ScreenshotError(String),
    /// Gemini API error
    GeminiError(String),
    /// Automation error
    AutomationError(String),
    /// Execution failed
    ExecutionError { step: usize, reason: String },
    /// Configuration error
    ConfigError(String),
    /// Platform not supported
    PlatformNotSupported,
}

impl From<CallError> for ServiceError {
    fn from(err: CallError) -> Self {
        ServiceError::Call(format!("{}", err))
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::Call(msg) => write!(f, "RPC call error: {}", msg),
            ServiceError::WindowNotFound(msg) => write!(f, "Window not found: {}", msg),
            ServiceError::ScreenshotError(msg) => write!(f, "Screenshot error: {}", msg),
            ServiceError::GeminiError(msg) => write!(f, "Gemini error: {}", msg),
            ServiceError::AutomationError(msg) => write!(f, "Automation error: {}", msg),
            ServiceError::ExecutionError { step, reason } => {
                write!(f, "Execution failed at step {}: {}", step, reason)
            }
            ServiceError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            ServiceError::PlatformNotSupported => write!(f, "Platform not supported"),
        }
    }
}

impl std::error::Error for ServiceError {}

/// Request to list windows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListWindowsRequest {
    pub executable_filter: Option<String>,
    pub title_pattern: Option<String>,
}

/// Request to take a screenshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotRequest {
    pub hwnd: String,
    pub method: Option<String>,
}

/// Screenshot result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub base64_image: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

/// Request to execute instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub hwnd: String,
    pub instructions: Vec<String>,
    pub retry_strategy: Option<String>,
}

/// Execution summary result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub total_steps: usize,
    pub completed_steps: usize,
    pub success: bool,
    pub last_error: Option<String>,
    pub coordinates: Vec<(i32, i32)>,
}

/// Request to detect elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectRequest {
    pub hwnd: String,
    pub query: String,
}

/// Element detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub element_found: bool,
    pub label: String,
    pub bounding_box: Option<[i32; 4]>,
    pub action_type: String,
    pub confidence: f32,
}
