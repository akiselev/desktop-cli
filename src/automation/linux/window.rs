//! X11 window enumeration and information
//!
//! Uses x11rb to query _NET_CLIENT_LIST for visible windows. Parallels
//! Windows EnumWindows but via X11 protocol instead of Win32 API.

use crate::automation::types::{WindowInfo, WindowRect};
use crate::error::{DesktopCliError, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

/// Parse window ID from hex string format
///
/// Window IDs are formatted as "0x{hex}" to match Windows HWND convention.
/// This shared function ensures consistent parsing across all Linux modules.
pub fn parse_window_id(hwnd: &str) -> Result<u32> {
    u32::from_str_radix(hwnd.trim_start_matches("0x"), 16)
        .map_err(|e| DesktopCliError::Platform(format!("Invalid window ID '{}': {}", hwnd, e)))
}

pub fn list_windows(
    exe_filter: Option<&str>,
    title_filter: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    let (conn, screen_num) = RustConnection::connect(None).map_err(|e| {
        crate::error::DesktopCliError::Platform(format!("X11 connection failed: {}", e))
    })?;

    let screen = &conn.setup().roots[screen_num];
    let net_client_list = conn
        .intern_atom(false, b"_NET_CLIENT_LIST")
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to intern atom: {}", e))
        })?
        .reply()
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get atom reply: {}", e))
        })?
        .atom;

    let property = conn
        .get_property(
            false,
            screen.root,
            net_client_list,
            AtomEnum::WINDOW,
            0,
            u32::MAX,
        )
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get property: {}", e))
        })?
        .reply()
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get property reply: {}", e))
        })?;

    // :UNSAFE: X11 property pointer cast to u32 slice; X11 protocol guarantees format=32 means 4-byte aligned u32 array
    let windows: &[u32] = if property.format == 32 {
        unsafe {
            std::slice::from_raw_parts(
                property.value.as_ptr() as *const u32,
                property.value.len() / 4,
            )
        }
    } else {
        &[]
    };

    let mut result = Vec::new();
    for &window_id in windows {
        if let Ok(info) = get_window_info(&conn, window_id) {
            let matches_exe = exe_filter.is_none_or(|filter| {
                info.executable
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            });
            let matches_title = title_filter
                .is_none_or(|filter| info.title.to_lowercase().contains(&filter.to_lowercase()));

            if matches_exe && matches_title {
                result.push(info);
            }
        }
    }

    Ok(result)
}

/// Get window information for a single X11 window
///
/// Queries window properties via X11 protocol. Called during window enumeration
/// and direct window lookups by ID.
fn get_window_info(conn: &RustConnection, window_id: u32) -> Result<WindowInfo> {
    let net_wm_name = conn
        .intern_atom(false, b"_NET_WM_NAME")
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to intern atom: {}", e))
        })?
        .reply()
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get atom reply: {}", e))
        })?
        .atom;
    let net_wm_pid = conn
        .intern_atom(false, b"_NET_WM_PID")
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to intern atom: {}", e))
        })?
        .reply()
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get atom reply: {}", e))
        })?
        .atom;

    let title_prop = conn
        .get_property(false, window_id, net_wm_name, AtomEnum::ANY, 0, 1024)
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get property: {}", e))
        })?
        .reply()
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get property reply: {}", e))
        })?;
    let title = String::from_utf8_lossy(&title_prop.value).to_string();

    let pid_prop = conn
        .get_property(false, window_id, net_wm_pid, AtomEnum::CARDINAL, 0, 1)
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get property: {}", e))
        })?
        .reply()
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get property reply: {}", e))
        })?;
    let pid = if pid_prop.format == 32 && !pid_prop.value.is_empty() {
        u32::from_ne_bytes(pid_prop.value[0..4].try_into().unwrap())
    } else {
        0
    };

    let geometry = conn
        .get_geometry(window_id)
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get geometry: {}", e))
        })?
        .reply()
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get geometry reply: {}", e))
        })?;

    Ok(WindowInfo {
        hwnd: format!("0x{:x}", window_id),
        title,
        executable: format!("/proc/{}/exe", pid),
        rect: WindowRect {
            x: geometry.x as i32,
            y: geometry.y as i32,
            width: geometry.width as u32,
            height: geometry.height as u32,
        },
        pid,
        class_name: None,
    })
}

pub fn get_window_info_by_id(window_id: u32) -> Result<WindowInfo> {
    let (conn, _screen_num) = RustConnection::connect(None).map_err(|e| {
        crate::error::DesktopCliError::Platform(format!("X11 connection failed: {}", e))
    })?;

    get_window_info(&conn, window_id)
}

/// Get window title for AT-SPI2 matching
///
/// Returns the _NET_WM_NAME property for the given X11 window ID.
/// Used by atspi.rs to correlate X11 windows with AT-SPI2 accessible objects.
pub fn get_window_title(window_id: u32) -> Result<String> {
    let (conn, _screen_num) = RustConnection::connect(None).map_err(|e| {
        crate::error::DesktopCliError::Platform(format!("X11 connection failed: {}", e))
    })?;

    let net_wm_name = conn
        .intern_atom(false, b"_NET_WM_NAME")
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to intern atom: {}", e))
        })?
        .reply()
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get atom reply: {}", e))
        })?
        .atom;

    let title_prop = conn
        .get_property(false, window_id, net_wm_name, AtomEnum::ANY, 0, 1024)
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get property: {}", e))
        })?
        .reply()
        .map_err(|e| {
            crate::error::DesktopCliError::Platform(format!("Failed to get property reply: {}", e))
        })?;

    Ok(String::from_utf8_lossy(&title_prop.value).to_string())
}
