//! macOS window enumeration via Cocoa

use crate::automation::types::WindowInfo;
use crate::automation::types::WindowRect;
use crate::error::{DesktopCliError, Result};

/// Lists all visible windows, optionally filtered by executable and/or title.
///
/// Uses Core Graphics window list API with on-screen-only filter.
/// Filtering is case-insensitive using contains() match.
/// Returns empty vec when no windows match filters (follows Rust iterator convention).
#[cfg(target_os = "macos")]
pub fn list_windows(
    exe_filter: Option<&str>,
    title_filter: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::window::{kCGWindowListOptionOnScreenOnly, CGWindowListCopyWindowInfo};

    // Uses kCGWindowListOptionOnScreenOnly - only visible windows relevant for automation,
    // hidden/minimized not interactable. See Decision Log.
    let window_list = unsafe {
        CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, 0)
    };
    if window_list.is_null() {
        return Ok(vec![]);
    }

    let windows: CFArray<CFDictionary> = unsafe { CFArray::wrap_under_create_rule(window_list) };
    let mut result = Vec::new();

    for i in 0..windows.len() {
        let Some(window_dict) = windows.get(i) else { continue; };

        let Some(window_id) = get_dict_number(&window_dict, "kCGWindowNumber") else { continue; };
        let window_id = window_id as u32;
        let window_name = get_dict_string(&window_dict, "kCGWindowName").unwrap_or_default();
        let owner_name = get_dict_string(&window_dict, "kCGWindowOwnerName").unwrap_or_default();
        let owner_pid = get_dict_number(&window_dict, "kCGWindowOwnerPID").unwrap_or(0) as u32;

        if window_name.is_empty() {
            continue;
        }

        let bounds = get_window_bounds(&window_dict);

        // Case-insensitive filtering using to_lowercase().contains() pattern (conformance: matches Linux window.rs implementation)
        let matches_exe = exe_filter.is_none_or(|filter| {
            owner_name.to_lowercase().contains(&filter.to_lowercase())
        });
        let matches_title = title_filter.is_none_or(|filter| {
            window_name.to_lowercase().contains(&filter.to_lowercase())
        });

        if matches_exe && matches_title {
            result.push(WindowInfo {
                hwnd: format!("0x{:x}", window_id),
                title: window_name,
                executable: owner_name,
                rect: bounds,
                pid: owner_pid,
                class_name: None,
            });
        }
    }

    Ok(result)
}

/// Retrieves window info for a specific window ID.
///
/// Returns error if window ID not found in the on-screen window list.
#[cfg(target_os = "macos")]
pub fn get_window_info_by_id(window_id: u32) -> Result<WindowInfo> {
    use core_foundation::array::CFArray;
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_graphics::window::{kCGWindowListOptionOnScreenOnly, CGWindowListCopyWindowInfo};

    let window_list = unsafe {
        CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, 0)
    };
    if window_list.is_null() {
        return Err(DesktopCliError::Platform(
            "Failed to retrieve window list from Core Graphics".to_string(),
        ));
    }

    let windows: CFArray<CFDictionary> = unsafe { CFArray::wrap_under_create_rule(window_list) };

    for i in 0..windows.len() {
        let Some(window_dict) = windows.get(i) else { continue; };
        let Some(wid) = get_dict_number(&window_dict, "kCGWindowNumber") else { continue; };
        let wid = wid as u32;

        if wid == window_id {
            let window_name = get_dict_string(&window_dict, "kCGWindowName").unwrap_or_default();
            let owner_name = get_dict_string(&window_dict, "kCGWindowOwnerName").unwrap_or_default();
            let owner_pid = get_dict_number(&window_dict, "kCGWindowOwnerPID").unwrap_or(0) as u32;
            let bounds = get_window_bounds(&window_dict);

            return Ok(WindowInfo {
                hwnd: format!("0x{:x}", window_id),
                title: window_name,
                executable: owner_name,
                rect: bounds,
                pid: owner_pid,
                class_name: None,
            });
        }
    }

    Err(DesktopCliError::WindowNotFound(format!(
        "Window with ID 0x{:x} not found",
        window_id
    )))
}

/// Extracts string value from CFDictionary by key.
#[cfg(target_os = "macos")]
fn get_dict_string(dict: &core_foundation::dictionary::CFDictionary, key: &str) -> Option<String> {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;

    let key_cf = CFString::new(key);
    dict.find(&key_cf.as_CFType())
        .and_then(|value| unsafe { CFString::wrap_under_get_rule(*value as _).as_CFTypeRef() as *const _ })
        .map(|s: *const _| unsafe { CFString::wrap_under_get_rule(s).to_string() })
}

/// Extracts number value from CFDictionary by key.
#[cfg(target_os = "macos")]
fn get_dict_number(dict: &core_foundation::dictionary::CFDictionary, key: &str) -> Option<i64> {
    use core_foundation::base::TCFType;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;

    let key_cf = CFString::new(key);
    dict.find(&key_cf.as_CFType())
        .and_then(|value| unsafe { CFNumber::wrap_under_get_rule(*value as _).to_i64() })
}

/// Extracts window bounds from CFDictionary as WindowRect.
#[cfg(target_os = "macos")]
fn get_window_bounds(dict: &core_foundation::dictionary::CFDictionary) -> WindowRect {
    use core_foundation::base::TCFType;
    use core_foundation::dictionary::CFDictionary as CFDict;
    use core_foundation::string::CFString;

    let key_cf = CFString::new("kCGWindowBounds");
    if let Some(bounds_ref) = dict.find(&key_cf.as_CFType()) {
        let bounds_dict: CFDict = unsafe { CFDict::wrap_under_get_rule(*bounds_ref as _) };

        let x = get_dict_number(&bounds_dict, "X").unwrap_or(0) as i32;
        let y = get_dict_number(&bounds_dict, "Y").unwrap_or(0) as i32;
        let width = get_dict_number(&bounds_dict, "Width").unwrap_or(0) as i32;
        let height = get_dict_number(&bounds_dict, "Height").unwrap_or(0) as i32;

        if width >= 0 && height >= 0 {
            return WindowRect { x, y, width: width as u32, height: height as u32 };
        }
    }

    WindowRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn list_windows(
    _exe_filter: Option<&str>,
    _title_filter: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    Err(crate::error::DesktopCliError::Platform(
        "macOS not supported on this platform".to_string(),
    ))
}
