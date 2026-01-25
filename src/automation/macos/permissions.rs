//! macOS accessibility permission detection

#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    // TODO: Implement using accessibility-sys crate
    // use accessibility_sys::AXIsProcessTrusted;
    // AXIsProcessTrusted() returns bool
    //
    // For now, return false and callers should handle with informative error:
    // "Desktop automation requires accessibility permissions.
    //  Grant access in System Preferences > Privacy & Security > Accessibility."
    false
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    false
}
