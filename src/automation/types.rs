use serde::{Deserialize, Serialize};

/// Platform-agnostic window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub hwnd: String,  // String representation for cross-platform compatibility
    pub title: String,
    pub executable: String,
    pub rect: WindowRect,
}

/// Window rectangle coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Actions that can be performed on a window
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    Click { x: i32, y: i32 },
    Type { text: String },
    Scroll { amount: i32 },
    KeyPress { key: String },
}
