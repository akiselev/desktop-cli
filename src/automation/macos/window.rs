//! macOS window enumeration and management via Core Graphics

use crate::automation::types::{WindowInfo, WindowRect};
use crate::error::{DesktopCliError, Result};
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::display::{
    kCGNullWindowID, kCGWindowListExcludeDesktopElements, kCGWindowListOptionOnScreenOnly,
    CGWindowListCopyWindowInfo,
};
use regex::Regex;
use std::collections::HashSet;

/// Window ID type for macOS (CGWindowID)
pub type WindowId = u32;

// Window dictionary keys
const WINDOW_NUMBER: &str = "kCGWindowNumber";
const WINDOW_OWNER_PID: &str = "kCGWindowOwnerPID";
const WINDOW_NAME: &str = "kCGWindowName";
const WINDOW_OWNER_NAME: &str = "kCGWindowOwnerName";
const WINDOW_BOUNDS: &str = "kCGWindowBounds";
const WINDOW_LAYER: &str = "kCGWindowLayer";

/// Get the parent process ID for a given process
fn get_parent_pid(pid: u32) -> Option<u32> {
    // On macOS, we can use sysctl or libproc
    // For simplicity, using ps command via process
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "ppid="])
        .output()
        .ok()?;

    if output.status.success() {
        let ppid_str = String::from_utf8_lossy(&output.stdout);
        ppid_str.trim().parse().ok()
    } else {
        None
    }
}

/// Collect all ancestor PIDs (parent, grandparent, etc.) up to a reasonable limit
fn get_ancestor_pids(start_pid: u32) -> HashSet<u32> {
    let mut ancestors = HashSet::new();
    ancestors.insert(start_pid);

    let mut current = start_pid;
    // Walk up to 10 levels to avoid infinite loops
    for _ in 0..10 {
        match get_parent_pid(current) {
            Some(parent) if parent > 1 && parent != current && !ancestors.contains(&parent) => {
                ancestors.insert(parent);
                current = parent;
            }
            _ => break,
        }
    }

    ancestors
}

/// Get the executable path for a process
fn get_process_executable(pid: u32) -> Option<String> {
    // Use libproc to get the executable path
    let mut path_buf = vec![0u8; 4096];
    unsafe {
        let ret = libc::proc_pidpath(
            pid as i32,
            path_buf.as_mut_ptr() as *mut libc::c_void,
            path_buf.len() as u32,
        );
        if ret > 0 {
            path_buf.truncate(ret as usize);
            String::from_utf8(path_buf).ok()
        } else {
            None
        }
    }
}

/// Helper to get a string value from a dictionary
fn get_dict_string(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<String> {
    dict.find(&CFString::new(key))
        .and_then(|v| v.downcast::<CFString>())
        .map(|s| s.to_string())
}

/// Helper to get an integer value from a dictionary
fn get_dict_i64(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<i64> {
    dict.find(&CFString::new(key))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_i64())
}

/// Helper to get a float value from a dictionary
fn get_dict_f64(dict: &CFDictionary<CFString, CFType>, key: &str) -> Option<f64> {
    dict.find(&CFString::new(key))
        .and_then(|v| v.downcast::<CFNumber>())
        .and_then(|n| n.to_f64())
}

/// Get window info from a CGWindow dictionary
fn window_info_from_dict(dict: &CFDictionary<CFString, CFType>) -> Option<WindowInfo> {
    // Get window ID
    let window_id = get_dict_i64(dict, WINDOW_NUMBER)? as u32;

    // Get window owner PID
    let pid = get_dict_i64(dict, WINDOW_OWNER_PID)? as u32;

    // Get window title (may be empty)
    let title = get_dict_string(dict, WINDOW_NAME).unwrap_or_default();

    // Get window owner name (application name)
    let app_name = get_dict_string(dict, WINDOW_OWNER_NAME).unwrap_or_default();

    // Get window bounds
    let bounds = dict
        .find(&CFString::new(WINDOW_BOUNDS))
        .and_then(|v| v.downcast::<CFDictionary<CFString, CFType>>())
        .map(|bounds_dict| {
            let x = get_dict_f64(&bounds_dict, "X").unwrap_or(0.0) as i32;
            let y = get_dict_f64(&bounds_dict, "Y").unwrap_or(0.0) as i32;
            let width = get_dict_f64(&bounds_dict, "Width").unwrap_or(0.0) as u32;
            let height = get_dict_f64(&bounds_dict, "Height").unwrap_or(0.0) as u32;

            WindowRect {
                x,
                y,
                width,
                height,
            }
        })
        .unwrap_or(WindowRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        });

    // Get executable path
    let executable = get_process_executable(pid).unwrap_or_else(|| app_name.clone());

    Some(WindowInfo {
        hwnd: format!("{}", window_id),
        title,
        executable,
        rect: bounds,
        pid,
        class_name: Some(app_name),
    })
}

/// List all visible windows, optionally filtered by executable and/or title pattern.
///
/// By default, windows belonging to the current process and its ancestor processes
/// (e.g., the terminal running this CLI) are excluded to prevent self-matching.
pub fn list_windows(
    exe_filter: Option<&str>,
    title_pattern: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    list_windows_impl(exe_filter, title_pattern, true)
}

/// List all visible windows, with option to include own process windows.
pub fn list_windows_include_self(
    exe_filter: Option<&str>,
    title_pattern: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    list_windows_impl(exe_filter, title_pattern, false)
}

fn list_windows_impl(
    exe_filter: Option<&str>,
    title_pattern: Option<&str>,
    exclude_own_process: bool,
) -> Result<Vec<WindowInfo>> {
    // Collect our own PID and all ancestor PIDs
    let excluded_pids = if exclude_own_process {
        get_ancestor_pids(std::process::id())
    } else {
        HashSet::new()
    };

    // Compile regex pattern if provided
    let title_regex = title_pattern
        .map(|p| Regex::new(p))
        .transpose()
        .map_err(|e| DesktopCliError::ConfigError(format!("Invalid regex pattern: {}", e)))?;

    // Get window list from Core Graphics
    let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
    let window_list_cf = unsafe { CGWindowListCopyWindowInfo(options, kCGNullWindowID) };

    if window_list_cf.is_null() {
        return Err(DesktopCliError::AutomationError(
            "Failed to get window list".to_string(),
        ));
    }

    // Convert to CFArray
    let window_array: CFArray<CFDictionary<CFString, CFType>> =
        unsafe { TCFType::wrap_under_create_rule(window_list_cf) };

    let mut windows = Vec::new();

    for i in 0..window_array.len() {
        if let Some(dict) = window_array.get(i) {
            // Skip dock/menu bar windows (layer != 0 indicates special windows)
            let layer = get_dict_i64(&dict, WINDOW_LAYER).unwrap_or(0);

            if layer != 0 {
                continue;
            }

            if let Some(info) = window_info_from_dict(&dict) {
                // Skip windows from our own process tree
                if excluded_pids.contains(&info.pid) {
                    continue;
                }

                // Skip windows without titles (usually not interesting)
                if info.title.is_empty() {
                    continue;
                }

                // Apply executable filter
                if let Some(exe) = exe_filter {
                    if !info.executable.to_lowercase().contains(&exe.to_lowercase()) {
                        continue;
                    }
                }

                // Apply title filter
                if let Some(ref regex) = title_regex {
                    if !regex.is_match(&info.title) {
                        continue;
                    }
                }

                windows.push(info);
            }
        }
    }

    Ok(windows)
}

/// Parse window ID from string representation
pub fn parse_window_id(window_str: &str) -> Result<WindowId> {
    window_str
        .parse::<u32>()
        .map_err(|e| DesktopCliError::AutomationError(format!("Invalid window ID: {}", e)))
}

/// Get window info for a specific window ID
pub fn get_window_info(window_id: WindowId) -> Result<WindowInfo> {
    // Get window list from Core Graphics
    let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
    let window_list_cf = unsafe { CGWindowListCopyWindowInfo(options, kCGNullWindowID) };

    if window_list_cf.is_null() {
        return Err(DesktopCliError::AutomationError(
            "Failed to get window list".to_string(),
        ));
    }

    let window_array: CFArray<CFDictionary<CFString, CFType>> =
        unsafe { TCFType::wrap_under_create_rule(window_list_cf) };

    for i in 0..window_array.len() {
        if let Some(dict) = window_array.get(i) {
            let wid = get_dict_i64(&dict, WINDOW_NUMBER).unwrap_or(0) as u32;

            if wid == window_id {
                if let Some(info) = window_info_from_dict(&dict) {
                    return Ok(info);
                }
            }
        }
    }

    Err(DesktopCliError::WindowNotFound(format!(
        "Window {} not found",
        window_id
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_window_id() {
        let wid = parse_window_id("12345").unwrap();
        assert_eq!(wid, 12345);
    }
}
