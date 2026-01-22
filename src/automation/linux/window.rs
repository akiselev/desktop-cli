//! Linux window enumeration and management via X11
//!
//! # Thread Safety
//!
//! X11 connections are NOT thread-safe. Each thread that needs to communicate
//! with the X server must have its own connection (Display pointer). The
//! `X11Connection` wrapper in this module is intentionally `!Send` and `!Sync`
//! to prevent accidental sharing across threads. If you need multi-threaded
//! access to X11, create separate connections in each thread.

use crate::automation::types::{WindowInfo, WindowRect};
use crate::error::{DesktopCliError, Result};
use regex::Regex;
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_char;
use x11::xlib::{
    self, Display, Window, XCloseDisplay, XDefaultRootWindow, XFetchName, XFree,
    XGetWindowAttributes, XGetWindowProperty, XInternAtom, XOpenDisplay, XQueryTree,
    XWindowAttributes,
};

/// X11 connection wrapper for safe resource management
pub struct X11Connection {
    display: *mut Display,
}

impl X11Connection {
    /// Open a connection to the X11 display
    pub fn new() -> Result<Self> {
        let display = unsafe { XOpenDisplay(std::ptr::null()) };
        if display.is_null() {
            return Err(DesktopCliError::AutomationError(
                "Failed to open X11 display. Is DISPLAY set?".to_string(),
            ));
        }
        Ok(Self { display })
    }

    /// Get the display pointer
    pub fn display(&self) -> *mut Display {
        self.display
    }

    /// Get the root window
    pub fn root_window(&self) -> Window {
        unsafe { XDefaultRootWindow(self.display) }
    }
}

impl Drop for X11Connection {
    fn drop(&mut self) {
        if !self.display.is_null() {
            unsafe { XCloseDisplay(self.display) };
        }
    }
}

// Note: X11Connection is automatically !Send and !Sync because *mut Display is !Send and !Sync
// This is intentional as X11 connections are not thread-safe and must be used from a single thread.

/// RAII wrapper for XQueryTree children array to ensure XFree is called even on panic
struct ChildrenGuard {
    ptr: *mut Window,
}

impl ChildrenGuard {
    fn new(ptr: *mut Window) -> Self {
        Self { ptr }
    }

    fn as_slice(&self, len: usize) -> &[Window] {
        if self.ptr.is_null() || len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, len) }
        }
    }
}

impl Drop for ChildrenGuard {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { XFree(self.ptr as *mut _) };
        }
    }
}

/// Get the parent process ID for a given process
fn get_parent_pid(pid: u32) -> Option<u32> {
    let stat_path = format!("/proc/{}/stat", pid);
    if let Ok(content) = std::fs::read_to_string(&stat_path) {
        // Format: pid (comm) state ppid ...
        // Find closing paren to skip command name which may contain spaces
        if let Some(paren_end) = content.rfind(')') {
            let after_paren = &content[paren_end + 2..];
            let parts: Vec<&str> = after_paren.split_whitespace().collect();
            if parts.len() > 1 {
                // ppid is after state
                return parts.get(1).and_then(|s| s.parse().ok());
            }
        }
    }
    None
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
    let exe_path = format!("/proc/{}/exe", pid);
    std::fs::read_link(&exe_path)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

/// Get the window title from X11
fn get_window_title(display: *mut Display, window: Window) -> String {
    unsafe {
        // Try _NET_WM_NAME first (UTF-8)
        let net_wm_name = XInternAtom(display, b"_NET_WM_NAME\0".as_ptr() as *const c_char, 0);
        let utf8_string = XInternAtom(display, b"UTF8_STRING\0".as_ptr() as *const c_char, 0);

        let mut actual_type: x11::xlib::Atom = 0;
        let mut actual_format: i32 = 0;
        let mut nitems: u64 = 0;
        let mut bytes_after: u64 = 0;
        let mut prop: *mut u8 = std::ptr::null_mut();

        let result = XGetWindowProperty(
            display,
            window,
            net_wm_name,
            0,
            1024,
            0,
            utf8_string,
            &mut actual_type,
            &mut actual_format,
            &mut nitems,
            &mut bytes_after,
            &mut prop,
        );

        if result == 0 && !prop.is_null() && nitems > 0 {
            let title = std::str::from_utf8(std::slice::from_raw_parts(prop, nitems as usize))
                .unwrap_or("")
                .to_string();
            XFree(prop as *mut _);
            if !title.is_empty() {
                return title;
            }
        }

        // Fallback to WM_NAME
        let mut name: *mut c_char = std::ptr::null_mut();
        if XFetchName(display, window, &mut name) != 0 && !name.is_null() {
            let title = CStr::from_ptr(name).to_string_lossy().to_string();
            XFree(name as *mut _);
            return title;
        }

        String::new()
    }
}

/// Get the PID of a window
fn get_window_pid(display: *mut Display, window: Window) -> Option<u32> {
    unsafe {
        let net_wm_pid = XInternAtom(display, b"_NET_WM_PID\0".as_ptr() as *const c_char, 0);
        let cardinal = XInternAtom(display, b"CARDINAL\0".as_ptr() as *const c_char, 0);

        let mut actual_type: x11::xlib::Atom = 0;
        let mut actual_format: i32 = 0;
        let mut nitems: u64 = 0;
        let mut bytes_after: u64 = 0;
        let mut prop: *mut u8 = std::ptr::null_mut();

        let result = XGetWindowProperty(
            display,
            window,
            net_wm_pid,
            0,
            1,
            0,
            cardinal,
            &mut actual_type,
            &mut actual_format,
            &mut nitems,
            &mut bytes_after,
            &mut prop,
        );

        if result == 0 && !prop.is_null() && nitems > 0 {
            let pid = *(prop as *const u32);
            XFree(prop as *mut _);
            return Some(pid);
        }

        None
    }
}

/// Get window class name (WM_CLASS)
fn get_window_class(display: *mut Display, window: Window) -> Option<String> {
    unsafe {
        let wm_class = XInternAtom(display, b"WM_CLASS\0".as_ptr() as *const c_char, 0);

        let mut actual_type: x11::xlib::Atom = 0;
        let mut actual_format: i32 = 0;
        let mut nitems: u64 = 0;
        let mut bytes_after: u64 = 0;
        let mut prop: *mut u8 = std::ptr::null_mut();

        let result = XGetWindowProperty(
            display,
            window,
            wm_class,
            0,
            1024,
            0,
            x11::xlib::AnyPropertyType as u64,
            &mut actual_type,
            &mut actual_format,
            &mut nitems,
            &mut bytes_after,
            &mut prop,
        );

        if result == 0 && !prop.is_null() && nitems > 0 {
            // WM_CLASS is two null-terminated strings
            let slice = std::slice::from_raw_parts(prop, nitems as usize);
            // Find the second string (class name)
            if let Some(null_pos) = slice.iter().position(|&b| b == 0) {
                if null_pos + 1 < slice.len() {
                    let class_name = std::str::from_utf8(&slice[null_pos + 1..])
                        .unwrap_or("")
                        .trim_end_matches('\0')
                        .to_string();
                    XFree(prop as *mut _);
                    if !class_name.is_empty() {
                        return Some(class_name);
                    }
                }
            }
            XFree(prop as *mut _);
        }

        None
    }
}

/// Get window geometry
fn get_window_geometry(display: *mut Display, window: Window) -> Option<WindowRect> {
    unsafe {
        let mut attrs: XWindowAttributes = std::mem::zeroed();
        if XGetWindowAttributes(display, window, &mut attrs) != 0 {
            Some(WindowRect {
                x: attrs.x,
                y: attrs.y,
                width: attrs.width as u32,
                height: attrs.height as u32,
            })
        } else {
            None
        }
    }
}

/// Check if a window is visible
fn is_window_visible(display: *mut Display, window: Window) -> bool {
    unsafe {
        let mut attrs: XWindowAttributes = std::mem::zeroed();
        if XGetWindowAttributes(display, window, &mut attrs) != 0 {
            attrs.map_state == xlib::IsViewable
        } else {
            false
        }
    }
}

/// Recursively find all windows
///
/// Uses RAII pattern (ChildrenGuard) to ensure XFree is called even if a panic occurs
/// during recursion or other operations.
fn find_all_windows(display: *mut Display, window: Window, windows: &mut Vec<Window>) {
    unsafe {
        let mut root_return: Window = 0;
        let mut parent_return: Window = 0;
        let mut children: *mut Window = std::ptr::null_mut();
        let mut nchildren: u32 = 0;

        if XQueryTree(
            display,
            window,
            &mut root_return,
            &mut parent_return,
            &mut children,
            &mut nchildren,
        ) != 0
        {
            // Use RAII guard to ensure XFree is called even if we panic
            let guard = ChildrenGuard::new(children);
            let children_slice = guard.as_slice(nchildren as usize);

            for &child in children_slice {
                // Check if this window is visible and has a title
                if is_window_visible(display, child) {
                    let title = get_window_title(display, child);
                    if !title.is_empty() {
                        windows.push(child);
                    }
                }
                // Recurse into children
                find_all_windows(display, child, windows);
            }
            // guard is dropped here, calling XFree automatically
        }
    }
}

/// List all visible windows, optionally filtered by executable and/or title pattern.
///
/// By default, windows belonging to the current process and its ancestor processes
/// (e.g., the terminal running this CLI) are excluded to prevent self-matching.
pub fn list_windows(exe_filter: Option<&str>, title_pattern: Option<&str>) -> Result<Vec<WindowInfo>> {
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
    let conn = X11Connection::new()?;

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

    // Find all windows
    let mut all_windows = Vec::new();
    find_all_windows(conn.display(), conn.root_window(), &mut all_windows);

    // Convert to WindowInfo and filter
    let mut windows = Vec::new();
    for window in all_windows {
        let title = get_window_title(conn.display(), window);
        if title.is_empty() {
            continue;
        }

        let pid = get_window_pid(conn.display(), window).unwrap_or(0);

        // Skip windows from our own process tree
        if excluded_pids.contains(&pid) {
            continue;
        }

        let executable = get_process_executable(pid).unwrap_or_default();

        // Apply executable filter
        if let Some(exe) = exe_filter {
            if !executable.to_lowercase().contains(&exe.to_lowercase()) {
                continue;
            }
        }

        // Apply title filter
        if let Some(ref regex) = title_regex {
            if !regex.is_match(&title) {
                continue;
            }
        }

        let rect = get_window_geometry(conn.display(), window).unwrap_or(WindowRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        });

        let class_name = get_window_class(conn.display(), window);

        windows.push(WindowInfo {
            hwnd: format!("{}", window),
            title,
            executable,
            rect,
            pid,
            class_name,
        });
    }

    Ok(windows)
}

/// Parse window ID from string representation
pub fn parse_window_id(window_str: &str) -> Result<Window> {
    window_str
        .parse::<u64>()
        .map_err(|e| DesktopCliError::AutomationError(format!("Invalid window ID: {}", e)))
}

/// Get window info for a specific window ID
pub fn get_window_info(window: Window) -> Result<WindowInfo> {
    let conn = X11Connection::new()?;

    if !is_window_visible(conn.display(), window) {
        return Err(DesktopCliError::WindowNotFound(
            "Window is not visible".to_string(),
        ));
    }

    let title = get_window_title(conn.display(), window);
    let pid = get_window_pid(conn.display(), window).unwrap_or(0);
    let executable = get_process_executable(pid).unwrap_or_default();
    let rect = get_window_geometry(conn.display(), window).unwrap_or(WindowRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    });
    let class_name = get_window_class(conn.display(), window);

    Ok(WindowInfo {
        hwnd: format!("{}", window),
        title,
        executable,
        rect,
        pid,
        class_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_window_id() {
        let wid = parse_window_id("123456").unwrap();
        assert_eq!(wid, 123456);
    }

    #[test]
    fn test_list_windows_compiles() {
        // This test ensures the code compiles
        // Actual window enumeration tests require an X11 server
        let _ = list_windows(None, None);
    }
}
