//! Direct operation implementations
//!
//! This module contains the actual implementation of desktop automation operations,
//! extracted from the RPC service for direct invocation without a daemon.

pub mod traits;

pub use traits::DesktopPlatform;

// ============================================================================
// Platform-Specific Modules
// ============================================================================

#[cfg(windows)]
mod windows_ops;

#[cfg(windows)]
pub use windows_ops::{WindowsPlatform, OpsError, Result};
#[cfg(windows)]
pub use windows_ops::WindowsPlatform as Platform;
#[cfg(windows)]
pub use windows_ops::{
    list_windows_api as list_windows,
    get_window_by_hwnd_api as get_window_by_hwnd,
    take_screenshot_api as take_screenshot,
    dump_tree_api as dump_tree,
    find_elements_api as find_elements,
    element_exists_api as element_exists,
    invoke_pattern_api as invoke_pattern,
    get_summary_api as get_summary,
    query_elements_api as query_elements,
    click_api as click,
    type_text_api as type_text,
    send_keys_api as send_keys,
    scroll_api as scroll,
};

#[cfg(target_os = "macos")]
mod macos_ops;

#[cfg(target_os = "macos")]
pub use macos_ops::{MacOSPlatform, OpsError, Result};
#[cfg(target_os = "macos")]
pub use macos_ops::MacOSPlatform as Platform;

#[cfg(target_os = "linux")]
mod linux_ops;

#[cfg(target_os = "linux")]
pub use linux_ops::{LinuxPlatform, OpsError, Result};
#[cfg(target_os = "linux")]
pub use linux_ops::LinuxPlatform as Platform;
#[cfg(target_os = "linux")]
pub use linux_ops::{
    list_windows_api as list_windows,
    get_window_by_hwnd_api as get_window_by_hwnd,
    take_screenshot_api as take_screenshot,
    dump_tree_api as dump_tree,
    find_elements_api as find_elements,
    element_exists_api as element_exists,
    invoke_pattern_api as invoke_pattern,
    get_summary_api as get_summary,
    query_elements_api as query_elements,
    click_api as click,
    type_text_api as type_text,
    send_keys_api as send_keys,
    scroll_api as scroll,
};

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
mod stub_ops;

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub use stub_ops::*;
