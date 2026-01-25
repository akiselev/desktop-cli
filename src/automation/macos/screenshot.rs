//! Screenshot capture via xcap

use crate::error::Result;
use crate::rpc::types::Screenshot;

#[cfg(target_os = "macos")]
pub fn capture_window(window_ref: &str) -> Result<Screenshot> {
    // TODO: Implement using xcap crate with macOS support
    //
    // Steps:
    // 1. Check screen recording permission (required on macOS 10.15+)
    //    - Use CGPreflightScreenCaptureAccess to check
    //    - Return informative error if permission denied
    // 2. Parse window_ref to get window ID
    // 3. Use xcap::Window::from_id or CGWindowListCreateImage:
    //    - kCGWindowImageDefault options
    //    - kCGWindowImageBoundsIgnoreFraming to exclude window chrome
    // 4. Convert CGImage to PNG bytes:
    //    - Use image crate or Core Graphics bitmap context
    // 5. Encode as base64 for Screenshot.data field
    // 6. Set Screenshot.format = "png"
    // 7. Return Screenshot struct
    //
    // Note: Screen recording permission prompt only appears on first capture attempt
    let _ = window_ref;
    Err(crate::error::DesktopCliError::Platform(
        "macOS screenshot not yet implemented. Grant screen recording permission in System Preferences > Privacy & Security > Screen Recording.".to_string()
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn capture_window(_window_ref: &str) -> Result<Screenshot> {
    Err(crate::error::DesktopCliError::Platform(
        "macOS not supported on this platform".to_string(),
    ))
}
