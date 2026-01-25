//! Platform abstraction traits for desktop automation
//!
//! Trait-based dispatch enables compile-time platform selection while providing
//! a uniform API for testing and shared logic across Windows, Linux, and macOS.

use crate::automation::types::WindowInfo;
use crate::rpc::types::{PatternResult, QueryResult, Screenshot, UiaElement};

use super::Result;

/// Cross-platform desktop automation operations
///
/// Each platform (Windows, Linux, macOS) implements this trait with platform-specific
/// automation APIs. The trait provides a uniform interface while preserving zero-cost
/// compile-time dispatch.
pub trait DesktopPlatform {
    /// List all visible windows with optional filters
    ///
    /// # Arguments
    /// * `exe_filter` - Filter windows by executable name (case-insensitive substring match)
    /// * `title_filter` - Filter windows by title (case-insensitive substring match)
    fn list_windows(
        &self,
        exe_filter: Option<&str>,
        title_filter: Option<&str>,
    ) -> Result<Vec<WindowInfo>>;

    /// Get window information by platform-specific handle string
    ///
    /// Handle format is platform-specific: HWND string on Windows, X11 window ID on Linux,
    /// PID+element reference on macOS. All code outside platform modules treats this as opaque.
    fn get_window_by_hwnd(&self, hwnd: &str) -> Result<WindowInfo>;

    /// Capture screenshot of a window
    ///
    /// # Arguments
    /// * `hwnd` - Window handle string
    /// * `method` - Screenshot method (platform-specific, e.g., "dwm" on Windows)
    fn take_screenshot(&self, hwnd: &str, method: Option<&str>) -> Result<Screenshot>;

    /// Dump element tree from window root
    ///
    /// Returns normalized UiaElement tree with cross-platform control types.
    /// Coordinates are pixels relative to window origin with DPI handling internal.
    fn dump_tree(&self, hwnd: &str, max_depth: u32) -> Result<UiaElement>;

    /// Find elements matching selector
    ///
    /// # Arguments
    /// * `hwnd` - Window handle string
    /// * `selector` - Element selector (same syntax across platforms)
    /// * `find_all` - Return all matches (true) or first match (false)
    fn find_elements(&self, hwnd: &str, selector: &str, find_all: bool) -> Result<Vec<UiaElement>>;

    /// Check if element matching selector exists
    fn element_exists(&self, hwnd: &str, selector: &str) -> Result<bool>;

    /// Invoke accessibility pattern on element
    ///
    /// # Arguments
    /// * `hwnd` - Window handle string
    /// * `selector` - Element selector
    /// * `pattern` - Pattern name (e.g., "Invoke", "Value")
    /// * `action` - Pattern-specific action
    fn invoke_pattern(
        &self,
        hwnd: &str,
        selector: &str,
        pattern: &str,
        action: Option<&str>,
    ) -> Result<PatternResult>;

    /// Get visual summary of window or element
    fn get_summary(
        &self,
        hwnd: &str,
        selector: &str,
        include_invisible: bool,
        include_offscreen: bool,
        bbox: Option<[i32; 4]>,
        max_depth: u32,
        control_types: Option<Vec<String>>,
    ) -> Result<String>;

    /// Query elements with structured results
    fn query_elements(&self, hwnd: &str, selector: &str, find_all: bool) -> Result<QueryResult>;

    /// Click at element or coordinates
    fn click(&self, hwnd: &str, selector: &str, coords: Option<(i32, i32)>, button: Option<&str>) -> Result<()>;

    /// Type text into element
    fn type_text(&self, hwnd: &str, text: &str, selector: Option<&str>) -> Result<()>;

    /// Send key combination (e.g., "ctrl+c")
    fn send_keys(&self, keys: &str) -> Result<()>;

    /// Scroll window or element
    fn scroll(&self, direction: &str, amount: i32) -> Result<()>;
}
