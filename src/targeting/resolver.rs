//! Window resolution with element-aware disambiguation
//!
//! Resolves window queries to specific HWNDs, with smart disambiguation
//! when multiple windows match by checking element selectors.

use std::fmt;

use crate::automation::types::WindowInfo;
use crate::targeting::parser::{IndexSpec, WindowQuery};

#[cfg(windows)]
use windows::Win32::Foundation::HWND;

/// Error during window resolution
#[derive(Debug, Clone)]
pub enum ResolutionError {
    /// No windows matched the query
    NoWindowMatch {
        query: String,
    },
    /// Multiple windows matched and couldn't be disambiguated
    AmbiguousWindow {
        query: String,
        windows: Vec<WindowInfo>,
    },
    /// Multiple windows had the element (used with element disambiguation)
    AmbiguousElement {
        selector: String,
        windows: Vec<WindowInfo>,
    },
    /// No windows had the element
    NoElementMatch {
        selector: String,
        windows: Vec<WindowInfo>,
    },
    /// Invalid HWND format
    InvalidHwnd(String),
    /// Window with HWND not found
    HwndNotFound(String),
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolutionError::NoWindowMatch { query } => {
                write!(f, "No windows found matching '{}'", query)
            }
            ResolutionError::AmbiguousWindow { query, windows } => {
                writeln!(f, "Found {} windows matching '{}':", windows.len(), query)?;
                for (i, w) in windows.iter().enumerate() {
                    writeln!(
                        f,
                        "  [{}] {} - {} (hwnd:{}, pid:{})",
                        i + 1,
                        w.executable,
                        w.title,
                        w.hwnd,
                        w.pid
                    )?;
                }
                write!(f, "Tip: Use ':1', ':2', etc. or refine with 'title:...'")
            }
            ResolutionError::AmbiguousElement { selector, windows } => {
                writeln!(
                    f,
                    "Found '{}' in {} windows:",
                    selector,
                    windows.len()
                )?;
                for (i, w) in windows.iter().enumerate() {
                    writeln!(
                        f,
                        "  [{}] {} - {} (hwnd:{})",
                        i + 1,
                        w.executable,
                        w.title,
                        w.hwnd
                    )?;
                }
                write!(f, "Tip: Use ':1' or refine with 'title:...'")
            }
            ResolutionError::NoElementMatch { selector, windows } => {
                writeln!(
                    f,
                    "Element '{}' not found in any of {} matching windows:",
                    selector,
                    windows.len()
                )?;
                for w in windows {
                    writeln!(f, "  - {} ({})", w.title, w.hwnd)?;
                }
                Ok(())
            }
            ResolutionError::InvalidHwnd(h) => {
                write!(f, "Invalid HWND format: '{}'", h)
            }
            ResolutionError::HwndNotFound(h) => {
                write!(f, "Window with HWND {} not found", h)
            }
        }
    }
}

impl std::error::Error for ResolutionError {}

/// Result of resolving a window query
#[derive(Debug)]
pub enum ResolutionResult {
    /// Exactly one window matched
    Single(WindowInfo),
    /// Multiple windows matched
    Multiple(Vec<WindowInfo>),
    /// No windows matched
    None,
}

/// Filter windows based on a query
pub fn filter_windows(query: &WindowQuery, windows: &[WindowInfo]) -> Vec<WindowInfo> {
    windows
        .iter()
        .filter(|w| window_matches(query, w))
        .cloned()
        .collect()
}

/// Check if a single window matches the query
fn window_matches(query: &WindowQuery, window: &WindowInfo) -> bool {
    // Check index - handled separately in resolve_window
    // Here we just filter by other criteria

    // HWND exact match
    if let Some(ref hwnd) = query.hwnd {
        return hwnd_matches(hwnd, &window.hwnd);
    }

    // PID exact match
    if let Some(pid) = query.pid {
        if window.pid != pid {
            return false;
        }
    }

    // Exe filter (substring, case-insensitive)
    if let Some(ref exe) = query.exe {
        if !window.executable.to_lowercase().contains(exe) {
            return false;
        }
    }

    // Title filter (with wildcards)
    if let Some(ref title_pattern) = query.title {
        if !title_pattern.matches(&window.title) {
            return false;
        }
    }

    // Class filter
    if let Some(ref class) = query.class {
        if !window
            .class_name
            .as_ref()
            .map(|c| c.to_lowercase().contains(&class.to_lowercase()))
            .unwrap_or(false)
        {
            return false;
        }
    }

    // General "any" filter - matches exe OR title
    if let Some(ref any) = query.any {
        let matches_exe = window.executable.to_lowercase().contains(any);
        let matches_title = window.title.to_lowercase().contains(any);
        if !matches_exe && !matches_title {
            return false;
        }
    }

    true
}

fn hwnd_matches(query_hwnd: &str, window_hwnd: &str) -> bool {
    // Normalize both to compare
    let query_val = parse_hwnd_value(query_hwnd);
    let window_val = parse_hwnd_value(window_hwnd);

    match (query_val, window_val) {
        (Some(q), Some(w)) => q == w,
        _ => query_hwnd.to_lowercase() == window_hwnd.to_lowercase(),
    }
}

fn parse_hwnd_value(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse::<u64>().ok()
    }
}

/// Resolve a window query to a single window
pub fn resolve_window(
    query: &WindowQuery,
    windows: &[WindowInfo],
) -> Result<WindowInfo, ResolutionError> {
    // Handle index-based queries first
    if let Some(ref index) = query.index {
        return resolve_by_index(index, windows);
    }

    // Filter by other criteria
    let matches = filter_windows(query, windows);

    match matches.len() {
        0 => Err(ResolutionError::NoWindowMatch {
            query: query.to_string(),
        }),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(ResolutionError::AmbiguousWindow {
            query: query.to_string(),
            windows: matches,
        }),
    }
}

fn resolve_by_index(index: &IndexSpec, windows: &[WindowInfo]) -> Result<WindowInfo, ResolutionError> {
    if windows.is_empty() {
        return Err(ResolutionError::NoWindowMatch {
            query: format!("{:?}", index),
        });
    }

    match index {
        IndexSpec::Number(n) => {
            if *n == 0 || *n > windows.len() {
                Err(ResolutionError::NoWindowMatch {
                    query: format!(":{} (valid range: 1-{})", n, windows.len()),
                })
            } else {
                Ok(windows[n - 1].clone())
            }
        }
        IndexSpec::First => Ok(windows[0].clone()),
        IndexSpec::Last => Ok(windows[windows.len() - 1].clone()),
    }
}

/// Resolve with element-aware disambiguation
///
/// When multiple windows match the query, tries the element selector on each
/// and returns the window where the element exists (if unique).
#[cfg(windows)]
pub fn resolve_with_element<F>(
    query: &WindowQuery,
    element_selector: &str,
    windows: &[WindowInfo],
    element_exists_fn: F,
) -> Result<WindowInfo, ResolutionError>
where
    F: Fn(&str, &str) -> Result<bool, String>,
{
    // Handle index-based queries - no disambiguation needed
    if query.index.is_some() {
        return resolve_window(query, windows);
    }

    // Filter windows by query
    let matches = filter_windows(query, windows);

    match matches.len() {
        0 => Err(ResolutionError::NoWindowMatch {
            query: query.to_string(),
        }),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => {
            // Multiple windows - try element selector on each
            let mut windows_with_element = Vec::new();

            for window in &matches {
                match element_exists_fn(&window.hwnd, element_selector) {
                    Ok(true) => windows_with_element.push(window.clone()),
                    Ok(false) => {}
                    Err(_) => {} // Treat errors as "not found"
                }
            }

            match windows_with_element.len() {
                0 => Err(ResolutionError::NoElementMatch {
                    selector: element_selector.to_string(),
                    windows: matches,
                }),
                1 => Ok(windows_with_element.into_iter().next().unwrap()),
                _ => Err(ResolutionError::AmbiguousElement {
                    selector: element_selector.to_string(),
                    windows: windows_with_element,
                }),
            }
        }
    }
}

#[cfg(not(windows))]
pub fn resolve_with_element<F>(
    query: &WindowQuery,
    _element_selector: &str,
    windows: &[WindowInfo],
    _element_exists_fn: F,
) -> Result<WindowInfo, ResolutionError>
where
    F: Fn(&str, &str) -> Result<bool, String>,
{
    // On non-Windows, just use basic resolution
    resolve_window(query, windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_windows() -> Vec<WindowInfo> {
        vec![
            WindowInfo {
                hwnd: "0x1234".to_string(),
                title: "Altium Designer - PCB1.PcbDoc".to_string(),
                executable: "Altium.exe".to_string(),
                pid: 1000,
                class_name: Some("TfrmAltium".to_string()),
            },
            WindowInfo {
                hwnd: "0x5678".to_string(),
                title: "Altium Designer - Schematic1.SchDoc".to_string(),
                executable: "Altium.exe".to_string(),
                pid: 1000,
                class_name: Some("TfrmAltium".to_string()),
            },
            WindowInfo {
                hwnd: "0x9ABC".to_string(),
                title: "Untitled - Notepad".to_string(),
                executable: "notepad.exe".to_string(),
                pid: 2000,
                class_name: Some("Notepad".to_string()),
            },
        ]
    }

    #[test]
    fn test_resolve_by_index() {
        let windows = make_windows();

        let query = WindowQuery::parse(":1").unwrap();
        let result = resolve_window(&query, &windows).unwrap();
        assert_eq!(result.hwnd, "0x1234");

        let query = WindowQuery::parse(":last").unwrap();
        let result = resolve_window(&query, &windows).unwrap();
        assert_eq!(result.hwnd, "0x9ABC");
    }

    #[test]
    fn test_resolve_unique_exe() {
        let windows = make_windows();

        let query = WindowQuery::parse("notepad").unwrap();
        let result = resolve_window(&query, &windows).unwrap();
        assert_eq!(result.pid, 2000);
    }

    #[test]
    fn test_resolve_ambiguous() {
        let windows = make_windows();

        let query = WindowQuery::parse("altium").unwrap();
        let result = resolve_window(&query, &windows);
        assert!(matches!(result, Err(ResolutionError::AmbiguousWindow { .. })));
    }

    #[test]
    fn test_resolve_by_title() {
        let windows = make_windows();

        let query = WindowQuery::parse("title:PCB").unwrap();
        let result = resolve_window(&query, &windows).unwrap();
        assert!(result.title.contains("PCB"));
    }

    #[test]
    fn test_resolve_by_hwnd() {
        let windows = make_windows();

        let query = WindowQuery::parse("hwnd:0x5678").unwrap();
        let result = resolve_window(&query, &windows).unwrap();
        assert!(result.title.contains("Schematic"));
    }
}
