use serde::{Deserialize, Serialize};

/// Platform-agnostic window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    /// String representation of HWND for cross-platform compatibility
    pub hwnd: String,
    /// Window title
    pub title: String,
    /// Full path to executable
    pub executable: String,
    /// Window rectangle
    pub rect: WindowRect,
    /// Process ID
    pub pid: u32,
    /// Window class name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
}

/// Window rectangle coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
