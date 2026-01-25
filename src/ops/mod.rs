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
pub use windows_ops::WindowsPlatform as Platform;
#[cfg(windows)]
pub use windows_ops::{
    click_api as click, dump_tree_api as dump_tree, element_exists_api as element_exists,
    find_elements_api as find_elements, get_summary_api as get_summary,
    get_window_by_hwnd_api as get_window_by_hwnd, invoke_pattern_api as invoke_pattern,
    list_windows_api as list_windows, query_elements_api as query_elements, scroll_api as scroll,
    send_keys_api as send_keys, take_screenshot_api as take_screenshot, type_text_api as type_text,
};
#[cfg(windows)]
pub use windows_ops::{OpsError, Result, WindowsPlatform};

#[cfg(target_os = "macos")]
mod macos_ops;

#[cfg(target_os = "macos")]
pub use macos_ops::MacOSPlatform as Platform;
#[cfg(target_os = "macos")]
pub use macos_ops::{MacOSPlatform, OpsError, Result};

#[cfg(target_os = "linux")]
mod linux_ops;

#[cfg(target_os = "linux")]
pub use linux_ops::LinuxPlatform as Platform;
#[cfg(target_os = "linux")]
pub use linux_ops::{
    click_api as click, dump_tree_api as dump_tree, element_exists_api as element_exists,
    find_elements_api as find_elements, get_summary_api as get_summary,
    get_window_by_hwnd_api as get_window_by_hwnd, invoke_pattern_api as invoke_pattern,
    list_windows_api as list_windows, query_elements_api as query_elements, scroll_api as scroll,
    send_keys_api as send_keys, take_screenshot_api as take_screenshot, type_text_api as type_text,
};
#[cfg(target_os = "linux")]
pub use linux_ops::{LinuxPlatform, OpsError, Result};

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
mod stub_ops;

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
pub use stub_ops::*;
