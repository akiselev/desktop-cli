//! Screenshot capture (deferred)
//!
//! Screenshot functionality deferred to post-release.
//! Stub types and functions preserved for API stability with executor module.

use crate::rpc::types::Screenshot;

/// Screenshot capture method
#[derive(Debug, Clone, Default)]
pub enum ScreenshotMethod {
    #[default]
    Default,
}

/// Capture a screenshot of the specified window.
///
/// Currently returns an error as screenshot support is deferred.
#[cfg(windows)]
pub fn capture_screenshot(
    _hwnd: windows::Win32::Foundation::HWND,
    _method: ScreenshotMethod,
) -> crate::error::Result<Screenshot> {
    Err(crate::error::DesktopCliError::Platform(
        "Screenshot capture not yet implemented".to_string(),
    ))
}
