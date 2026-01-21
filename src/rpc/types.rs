use serde::{Deserialize, Serialize};

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

/// Screenshot result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    pub base64_image: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}
