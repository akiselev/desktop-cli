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

/// Request to list windows (filters applied at daemon start time)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListWindowsRequest {}

/// Request to set the default target window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetDefaultWindowRequest {
    /// Window index from list (1-based) or HWND string
    pub window: String,
}

/// Request to get the current default window
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GetDefaultWindowRequest {}

/// Response containing the default window info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultWindowResponse {
    pub hwnd: Option<String>,
    pub title: Option<String>,
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

// ============================================================================
// UIA (UI Automation) Types
// ============================================================================

/// Request to dump UIA element tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DumpTreeRequest {
    pub hwnd: String,
    /// Maximum depth (default: 5)
    pub max_depth: Option<u32>,
    /// Skip offscreen elements
    pub prune_offscreen: Option<bool>,
    /// Skip elements with empty name and no patterns
    pub prune_empty: Option<bool>,
    /// Max items per repeated container (lists, grids)
    pub max_list_items: Option<u32>,
}

/// Request to find elements by CSS-style selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindElementRequest {
    pub hwnd: String,
    /// CSS-style selector (e.g., "Button#save", "[name~='*OK*']")
    pub selector: String,
    /// Find all matches (default: first only)
    pub find_all: Option<bool>,
    /// Timeout in milliseconds (default: 3000)
    pub timeout_ms: Option<u64>,
}

/// Request to invoke a UIA pattern on an element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokePatternRequest {
    pub hwnd: String,
    /// CSS-style selector to find the element
    pub selector: String,
    /// Pattern operation (invoke, get-value, set-value, toggle, select, expand, collapse, etc.)
    pub pattern: String,
    /// Optional value for set operations
    pub value: Option<String>,
}

// ============================================================================
// UIA Element Types (cross-platform for RPC serialization)
// ============================================================================

/// Serializable representation of a UI Automation element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiaElement {
    /// Unique runtime ID for this session
    pub id: String,
    /// ControlType as string (Button, Edit, List, etc.)
    pub control_type: String,
    /// Localized control type name
    pub localized_type: String,
    /// Element name (visible text/label)
    pub name: String,
    /// Automation ID (stable developer-assigned ID)
    pub automation_id: String,
    /// ClassName
    pub class_name: String,
    /// Current value (from ValuePattern if available)
    pub value: Option<String>,
    /// Bounding rectangle [x, y, width, height]
    pub bounds: [i32; 4],
    /// Is element enabled?
    pub is_enabled: bool,
    /// Is element visible/on-screen?
    pub is_offscreen: bool,
    /// Supported pattern names (Invoke, Value, Toggle, etc.)
    pub patterns: Vec<String>,
    /// Nesting depth in tree
    pub depth: u32,
    /// Child elements (when tree dump includes children)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<UiaElement>,
}

impl Default for UiaElement {
    fn default() -> Self {
        Self {
            id: String::new(),
            control_type: String::new(),
            localized_type: String::new(),
            name: String::new(),
            automation_id: String::new(),
            class_name: String::new(),
            value: None,
            bounds: [0, 0, 0, 0],
            is_enabled: true,
            is_offscreen: false,
            patterns: Vec::new(),
            depth: 0,
            children: Vec::new(),
        }
    }
}

/// Options for tree dumping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeDumpOptions {
    /// Maximum depth to traverse (default: 5)
    pub max_depth: u32,
    /// Skip elements that are offscreen
    pub prune_offscreen: bool,
    /// Skip elements with empty name and no patterns
    pub prune_empty: bool,
    /// Max items per repeated container (0 = unlimited)
    pub max_list_items: u32,
}

impl Default for TreeDumpOptions {
    fn default() -> Self {
        Self {
            max_depth: 5,
            prune_offscreen: true,
            prune_empty: true,
            max_list_items: 20,
        }
    }
}

/// Result of a pattern invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternResult {
    pub success: bool,
    /// Current value after operation (if applicable)
    pub value: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
}

impl PatternResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            value: None,
            error: None,
        }
    }

    pub fn ok_with_value(value: String) -> Self {
        Self {
            success: true,
            value: Some(value),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            value: None,
            error: Some(msg.into()),
        }
    }
}

