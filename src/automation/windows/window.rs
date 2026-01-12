use crate::automation::types::{WindowInfo, WindowRect};
use crate::error::{DesktopMcpError, Result};
use regex::Regex;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible};

/// List all visible windows, optionally filtered by executable and/or title pattern
pub fn list_windows(
    exe_filter: Option<&str>,
    title_pattern: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    let mut windows = Vec::new();

    // Compile regex pattern if provided
    let title_regex = title_pattern
        .map(|p| Regex::new(p))
        .transpose()
        .map_err(|e| DesktopMcpError::ConfigError(format!("Invalid regex pattern: {}", e)))?;

    unsafe {
        EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut windows as *mut _ as isize),
        )
        .map_err(|e| DesktopMcpError::AutomationError(format!("EnumWindows failed: {}", e)))?;
    }

    // Apply filters
    let filtered = windows
        .into_iter()
        .filter(|w| {
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

        QueryFullProcessImageNameW(process_handle, 0, &mut exe_path_buf, &mut size)
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
        .map_err(|e| DesktopMcpError::AutomationError(format!("Invalid HWND: {}", e)))
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
