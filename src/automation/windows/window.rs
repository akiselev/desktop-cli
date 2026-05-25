use crate::automation::types::{WindowInfo, WindowRect};
use crate::error::{DesktopCliError, Result};
use regex::Regex;
use windows::core::PWSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_NAME_WIN32,
    PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

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

/// Get the parent process ID for a given process.
fn get_parent_pid(pid: u32) -> Option<u32> {
    use windows::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

        #[repr(C)]
        struct ProcessBasicInfo {
            reserved1: *mut std::ffi::c_void,
            peb_base_address: *mut std::ffi::c_void,
            reserved2: [*mut std::ffi::c_void; 2],
            unique_process_id: usize,
            inherited_from_unique_process_id: usize,
        }

        let mut info: ProcessBasicInfo = std::mem::zeroed();
        let mut return_length = 0u32;

        let status = NtQueryInformationProcess(
            handle,
            ProcessBasicInformation,
            &mut info as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<ProcessBasicInfo>() as u32,
            &mut return_length,
        );

        let _ = windows::Win32::Foundation::CloseHandle(handle);

        if status.is_ok() && info.inherited_from_unique_process_id != 0 {
            Some(info.inherited_from_unique_process_id as u32)
        } else {
            None
        }
    }
}

/// Collect all ancestor PIDs (parent, grandparent, etc.) up to a reasonable limit.
fn get_ancestor_pids(start_pid: u32) -> std::collections::HashSet<u32> {
    let mut ancestors = std::collections::HashSet::new();
    ancestors.insert(start_pid);

    let mut current = start_pid;
    // Walk up to 10 levels to avoid infinite loops from circular references
    for _ in 0..10 {
        match get_parent_pid(current) {
            Some(parent) if parent != 0 && parent != current && !ancestors.contains(&parent) => {
                ancestors.insert(parent);
                current = parent;
            }
            _ => break,
        }
    }

    ancestors
}

fn list_windows_impl(
    exe_filter: Option<&str>,
    title_pattern: Option<&str>,
    exclude_own_process: bool,
) -> Result<Vec<WindowInfo>> {
    let mut windows = Vec::new();

    // Collect our own PID and all ancestor PIDs (terminal, shell, etc.)
    let excluded_pids = if exclude_own_process {
        get_ancestor_pids(std::process::id())
    } else {
        std::collections::HashSet::new()
    };

    // Compile regex pattern if provided
    let title_regex = title_pattern
        .map(|p| Regex::new(p))
        .transpose()
        .map_err(|e| DesktopCliError::ConfigError(format!("Invalid regex pattern: {}", e)))?;

    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut windows as *mut _ as isize),
        )
        .map_err(|e| DesktopCliError::AutomationError(format!("EnumWindows failed: {}", e)))?;
    }

    // Apply filters
    let filtered = windows
        .into_iter()
        .filter(|w: &WindowInfo| {
            // Skip windows from our own process tree (CLI + terminal + shell ancestors)
            if excluded_pids.contains(&w.pid) {
                return false;
            }

            // Filter by executable path
            if let Some(exe) = exe_filter {
                if !w.executable.to_lowercase().contains(&exe.to_lowercase()) {
                    return false;
                }
            }

            // Filter by title pattern
            if let Some(ref regex) = title_regex {
                if !regex.is_match(&w.title) {
                    return false;
                }
            }

            true
        })
        .collect();

    Ok(filtered)
}

/// Win32 callback function for EnumWindows
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // Skip invisible windows
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1); // TRUE = continue enumeration
    }

    let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);

    // Get window title
    let mut title_buf = vec![0u16; 256];
    let title_len = GetWindowTextW(hwnd, &mut title_buf);
    let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

    // Skip windows without titles (usually not interesting)
    if title.is_empty() {
        return BOOL(1);
    }

    // Get process ID and executable path
    let mut process_id = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));

    let executable = get_process_executable(process_id).unwrap_or_default();

    // Skip if we couldn't get the executable (usually system processes)
    if executable.is_empty() {
        return BOOL(1);
    }

    // Get window class name
    let mut class_buf = vec![0u16; 256];
    let class_len = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut class_buf);
    let class_name = if class_len > 0 {
        Some(String::from_utf16_lossy(&class_buf[..class_len as usize]))
    } else {
        None
    };

    // Get window rect
    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);

    windows.push(WindowInfo {
        hwnd: format!("{}", hwnd.0 as isize),
        title,
        executable,
        rect: WindowRect {
            x: rect.left,
            y: rect.top,
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        },
        pid: process_id,
        class_name,
    });

    BOOL(1) // TRUE = continue enumeration
}

/// Get the executable path for a given process ID
fn get_process_executable(process_id: u32) -> Option<String> {
    unsafe {
        let process_handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            process_id,
        )
        .ok()?;

        let mut exe_path_buf = vec![0u16; 1024];
        let mut size = exe_path_buf.len() as u32;

        QueryFullProcessImageNameW(
            process_handle,
            PROCESS_NAME_WIN32,
            PWSTR::from_raw(exe_path_buf.as_mut_ptr()),
            &mut size,
        )
        .ok()?;

        let exe_path = String::from_utf16_lossy(&exe_path_buf[..size as usize]);
        Some(exe_path)
    }
}

/// Parse HWND from string representation
pub fn parse_hwnd(hwnd_str: &str) -> Result<HWND> {
    hwnd_str
        .parse::<isize>()
        .map(|h| HWND(h as _))
        .map_err(|e| DesktopCliError::AutomationError(format!("Invalid HWND: {}", e)))
}

/// Get window info for a specific HWND
pub fn get_window_info(hwnd: HWND) -> Result<WindowInfo> {
    unsafe {
        // Check if window is visible
        if !IsWindowVisible(hwnd).as_bool() {
            return Err(DesktopCliError::WindowNotFound(
                "Window is not visible".to_string(),
            ));
        }

        // Get window title
        let mut title_buf = vec![0u16; 256];
        let title_len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        // Get process ID and executable path
        let mut process_id = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        let executable = get_process_executable(process_id).unwrap_or_default();

        // Get window class name
        let mut class_buf = vec![0u16; 256];
        let class_len =
            windows::Win32::UI::WindowsAndMessaging::GetClassNameW(hwnd, &mut class_buf);
        let class_name = if class_len > 0 {
            Some(String::from_utf16_lossy(&class_buf[..class_len as usize]))
        } else {
            None
        };

        // Get window rect
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).map_err(|e| {
            DesktopCliError::AutomationError(format!("GetWindowRect failed: {}", e))
        })?;

        Ok(WindowInfo {
            hwnd: format!("{}", hwnd.0 as isize),
            title,
            executable,
            rect: WindowRect {
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left) as u32,
                height: (rect.bottom - rect.top) as u32,
            },
            pid: process_id,
            class_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_windows_compiles() {
        // This test just ensures the code compiles
        // Actual window enumeration tests would require a GUI environment
        let _ = list_windows(None, None);
    }

    #[test]
    fn test_parse_hwnd() {
        let hwnd = parse_hwnd("123456").unwrap();
        assert_eq!(hwnd.0 as isize, 123456);
    }
}
