use serde::{Deserialize, Serialize};

/// Error type for operations (previously RPC service calls)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceError {
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

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
    /// Optional post-action summary of UI state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<ActionSummary>,
}

impl PatternResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            value: None,
            error: None,
            summary: None,
        }
    }

    pub fn ok_with_value(value: String) -> Self {
        Self {
            success: true,
            value: Some(value),
            error: None,
            summary: None,
        }
    }

    pub fn ok_with_summary(summary: ActionSummary) -> Self {
        Self {
            success: true,
            value: None,
            error: None,
            summary: Some(summary),
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            value: None,
            error: Some(msg.into()),
            summary: None,
        }
    }
}

// ============================================================================
// Summary Types (LLM-optimized compact output)
// ============================================================================

/// Request to get UI summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryRequest {
    pub hwnd: String,
    /// Output format: "json" (default), "text", "compact"
    pub format: Option<String>,
    /// Include bounding boxes
    pub include_bounds: Option<bool>,
    /// Include full hierarchy paths
    pub include_paths: Option<bool>,
    /// Focus on region [x, y, width, height]
    pub focus_region: Option<[i32; 4]>,
    /// Maximum depth (default: 10)
    pub max_depth: Option<u32>,
    /// Filter by roles (e.g., ["button", "input"])
    pub roles: Option<Vec<String>>,
}

/// Post-action summary showing what changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSummary {
    /// What action was performed
    pub action: String,
    /// Target element description
    pub target: String,
    /// Whether the action succeeded
    pub success: bool,
    /// New focused element (if changed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_focus: Option<String>,
    /// Elements that appeared after action
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub appeared: Vec<String>,
    /// Elements that disappeared after action
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub disappeared: Vec<String>,
    /// Value changes (element -> new value)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub value_changes: Vec<(String, String)>,
    /// Nearby actionable elements (for context)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub nearby_actions: Vec<String>,
}

impl Default for ActionSummary {
    fn default() -> Self {
        Self {
            action: String::new(),
            target: String::new(),
            success: false,
            new_focus: None,
            appeared: Vec::new(),
            disappeared: Vec::new(),
            value_changes: Vec::new(),
            nearby_actions: Vec::new(),
        }
    }
}

// ============================================================================
// Enhanced Query Types
// ============================================================================

/// Request to query elements using enhanced selector syntax
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub hwnd: String,
    /// Query string (see QuerySyntax for format)
    pub query: String,
    /// Return all matches (default: first only)
    pub all: Option<bool>,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
    /// Output format: "full", "compact", "refs"
    pub format: Option<String>,
}

/// Compact element reference for query results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementRef {
    /// Short reference ID (e.g., "b1", "i3")
    pub id: String,
    /// Semantic role
    pub role: String,
    /// Display label
    pub label: String,
    /// Available action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Selector that uniquely identifies this element
    pub selector: String,
}

/// Query result with matches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Number of matches found
    pub count: usize,
    /// Matched elements
    pub matches: Vec<ElementRef>,
    /// Suggested selectors for similar elements
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub suggestions: Vec<String>,
}

// ============================================================================
// Input Action Types
// ============================================================================

/// Request to click at coordinates or on an element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickRequest {
    pub hwnd: String,
    /// Click type: "left", "right", "double"
    #[serde(default = "default_click_type")]
    pub click_type: String,
    /// Click at these window-relative coordinates (x, y)
    pub coords: Option<(i32, i32)>,
    /// Or click on element matching this selector
    pub selector: Option<String>,
}

fn default_click_type() -> String {
    "left".to_string()
}

/// Request to type text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeTextRequest {
    pub hwnd: String,
    /// Text to type
    pub text: String,
    /// Optional: selector to focus first
    pub selector: Option<String>,
}

/// Request to send key combination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendKeysRequest {
    pub hwnd: String,
    /// Key combination like "ctrl+c", "alt+f4", "enter", "tab"
    pub keys: String,
}

/// Request to scroll
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollRequest {
    pub hwnd: String,
    /// Scroll direction: "up", "down"
    pub direction: String,
    /// Number of scroll notches (default: 3)
    #[serde(default = "default_scroll_amount")]
    pub amount: i32,
    /// Optional: scroll at these coordinates
    pub coords: Option<(i32, i32)>,
}

fn default_scroll_amount() -> i32 {
    3
}

/// Generic action result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub error: Option<String>,
}

// ============================================================================
// Agent Types (LLM-driven automation)
// ============================================================================

/// Request to run the automation agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub hwnd: String,
    /// Natural language goal/instructions
    pub goal: String,
    /// Maximum steps before giving up (default: 20)
    #[serde(default = "default_max_steps")]
    pub max_steps: usize,
    /// Include screenshot in LLM context (default: true)
    #[serde(default = "default_true")]
    pub include_screenshot: bool,
    /// Include UI tree summary in LLM context (default: true)
    #[serde(default = "default_true")]
    pub include_ui_summary: bool,
}

fn default_max_steps() -> usize {
    20
}

fn default_true() -> bool {
    true
}

/// A step taken by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStepInfo {
    pub step: usize,
    pub reasoning: String,
    pub action: String,
    pub action_details: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Whether the goal was achieved
    pub success: bool,
    /// Summary of what happened
    pub summary: String,
    /// Number of steps taken
    pub steps_taken: usize,
    /// Final status: "done", "failed", "max_steps", "error"
    pub status: String,
    /// History of all steps
    pub history: Vec<AgentStepInfo>,
}

