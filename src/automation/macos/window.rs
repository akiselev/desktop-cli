//! macOS window enumeration via Cocoa

use crate::automation::types::WindowInfo;
use crate::error::Result;

#[cfg(target_os = "macos")]
pub fn list_windows(exe_filter: Option<&str>, title_filter: Option<&str>) -> Result<Vec<WindowInfo>> {
    // TODO: Implement using Core Graphics CGWindowListCopyWindowInfo
    // with accessibility_sys for window info
    //
    // Steps:
    // 1. Check permissions first using super::permissions::check_accessibility_permission()
    // 2. Call CGWindowListCopyWindowInfo with kCGWindowListOptionOnScreenOnly
    // 3. Filter by exe_filter/title_filter parameters
    // 4. For each window, get:
    //    - Window ID (kCGWindowNumber) -> convert to String for hwnd field
    //    - Window title (kCGWindowName)
    //    - Process ID (kCGWindowOwnerPID)
    //    - Bounds (kCGWindowBounds) -> map to WindowRect
    //    - Owner name (kCGWindowOwnerName) -> use for executable field
    // 5. Return Vec<WindowInfo>
    let _ = (exe_filter, title_filter);
    Err(crate::error::DesktopCliError::Platform(
        "macOS window listing not yet implemented. Grant accessibility permissions in System Preferences > Privacy & Security > Accessibility.".to_string()
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn list_windows(_exe_filter: Option<&str>, _title_filter: Option<&str>) -> Result<Vec<WindowInfo>> {
    Err(crate::error::DesktopCliError::Platform("macOS not supported on this platform".to_string()))
}
