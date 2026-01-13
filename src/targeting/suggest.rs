//! Query suggestion generation for agents
//!
//! Generates helpful query suggestions that uniquely identify windows,
//! helping agents discover what queries they can use.

use crate::automation::types::WindowInfo;

/// Suggestions for how to query a specific window
#[derive(Debug, Clone, Default)]
pub struct WindowQuerySuggestions {
    /// Queries that uniquely identify this window
    pub unique: Vec<String>,
    /// Queries that match multiple windows (query, count)
    pub shared: Vec<(String, usize)>,
}

/// Generate query suggestions for a specific window
pub fn suggest_queries(target_hwnd: &str, all_windows: &[WindowInfo]) -> Option<WindowQuerySuggestions> {
    let target = all_windows.iter().find(|w| w.hwnd == target_hwnd)?;
    let mut suggestions = WindowQuerySuggestions::default();

    // HWND is always unique
    suggestions.unique.push(format!("hwnd:{}", target.hwnd));

    // Find the index
    if let Some(idx) = all_windows.iter().position(|w| w.hwnd == target_hwnd) {
        suggestions.unique.push(format!(":{}", idx + 1));
    }

    // Check if exe name is unique
    let exe_base = extract_exe_name(&target.executable);
    let exe_matches = all_windows
        .iter()
        .filter(|w| w.executable.to_lowercase().contains(&exe_base.to_lowercase()))
        .count();

    if exe_matches == 1 {
        suggestions.unique.push(exe_base.clone());
    } else if exe_matches > 1 {
        suggestions.shared.push((exe_base.clone(), exe_matches));

        // Try to find a unique title component
        if let Some(unique_title) = find_unique_title_component(target, all_windows, &exe_base) {
            suggestions
                .unique
                .push(format!("{} title:{}", exe_base, unique_title));
        }
    }

    // Check for unique title patterns
    let title_keywords = extract_title_keywords(&target.title);
    for keyword in title_keywords {
        let matches = all_windows
            .iter()
            .filter(|w| w.title.to_lowercase().contains(&keyword.to_lowercase()))
            .count();

        if matches == 1 {
            suggestions.unique.push(format!("title:{}", keyword));
        } else if matches > 1 && matches < all_windows.len() {
            // Only add as shared if it's more specific than "all windows"
            suggestions.shared.push((format!("title:{}", keyword), matches));
        }
    }

    // PID-based query
    let pid_matches = all_windows.iter().filter(|w| w.pid == target.pid).count();
    if pid_matches == 1 {
        suggestions.unique.push(format!("pid:{}", target.pid));
    } else if pid_matches > 1 {
        suggestions.shared.push((format!("pid:{}", target.pid), pid_matches));
    }

    Some(suggestions)
}

/// Format window list with query hints for display
pub fn format_window_list(windows: &[WindowInfo]) -> String {
    let mut output = String::new();

    output.push_str(&format!("Windows ({} found):\n", windows.len()));

    for (i, window) in windows.iter().enumerate() {
        let exe_base = extract_exe_name(&window.executable);

        output.push_str(&format!(
            "  [:{idx}] {exe} | title:\"{title}\" | hwnd:{hwnd} | pid:{pid}\n",
            idx = i + 1,
            exe = exe_base,
            title = truncate(&window.title, 50),
            hwnd = window.hwnd,
            pid = window.pid,
        ));
    }

    if !windows.is_empty() {
        output.push_str("\nQuery examples:\n");

        // Show index example
        output.push_str(&format!(
            "  :1                    → {}\n",
            truncate(&windows[0].title, 30)
        ));

        // Show exe examples for unique exes
        let mut seen_exes = std::collections::HashSet::new();
        for window in windows {
            let exe_base = extract_exe_name(&window.executable);
            if seen_exes.insert(exe_base.clone()) {
                let exe_matches = windows
                    .iter()
                    .filter(|w| extract_exe_name(&w.executable) == exe_base)
                    .count();

                if exe_matches == 1 {
                    output.push_str(&format!(
                        "  {:<20}  → {}\n",
                        exe_base,
                        truncate(&window.title, 30)
                    ));
                }
            }
        }
    }

    output
}

/// Format window list as JSON for agents
pub fn format_window_list_json(windows: &[WindowInfo]) -> serde_json::Value {
    let window_entries: Vec<serde_json::Value> = windows
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let suggestions = suggest_queries(&w.hwnd, windows);
            serde_json::json!({
                "index": i + 1,
                "hwnd": w.hwnd,
                "pid": w.pid,
                "exe": w.executable,
                "title": w.title,
                "class": w.class_name,
                "unique_queries": suggestions.as_ref().map(|s| &s.unique).unwrap_or(&vec![]),
                "shared_queries": suggestions.as_ref().map(|s| {
                    s.shared.iter().map(|(q, c)| serde_json::json!({"query": q, "matches": c})).collect::<Vec<_>>()
                }).unwrap_or_default(),
            })
        })
        .collect();

    serde_json::json!({
        "windows": window_entries
    })
}

/// Format suggestions for a specific window
pub fn format_suggestions(hwnd: &str, windows: &[WindowInfo]) -> Option<String> {
    let target = windows.iter().find(|w| w.hwnd == hwnd)?;
    let suggestions = suggest_queries(hwnd, windows)?;

    let mut output = String::new();

    output.push_str(&format!("Window: {}\n\n", target.title));

    if !suggestions.unique.is_empty() {
        output.push_str("Unique selectors (will match only this window):\n");
        for q in &suggestions.unique {
            output.push_str(&format!("  {}\n", q));
        }
    }

    if !suggestions.shared.is_empty() {
        output.push_str("\nShared selectors (match multiple windows):\n");
        for (q, count) in &suggestions.shared {
            output.push_str(&format!("  {:<20} → matches {} windows\n", q, count));
        }
    }

    Some(output)
}

// Helper functions

fn extract_exe_name(exe_path: &str) -> String {
    exe_path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(exe_path)
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .to_lowercase()
}

fn extract_title_keywords(title: &str) -> Vec<String> {
    // Extract meaningful keywords from window title
    let mut keywords = Vec::new();

    // Split by common separators
    for part in title.split(['-', '–', '—', '|', ':']) {
        let part = part.trim();
        if part.len() >= 3 && part.len() <= 30 {
            keywords.push(part.to_string());
        }
    }

    // Also look for file extensions
    for word in title.split_whitespace() {
        if word.contains('.') && word.len() <= 30 {
            keywords.push(word.to_string());
        }
    }

    keywords
}

fn find_unique_title_component(
    target: &WindowInfo,
    all_windows: &[WindowInfo],
    exe_filter: &str,
) -> Option<String> {
    // Find windows with same exe
    let same_exe: Vec<_> = all_windows
        .iter()
        .filter(|w| w.executable.to_lowercase().contains(&exe_filter.to_lowercase()))
        .collect();

    if same_exe.len() <= 1 {
        return None;
    }

    // Find unique keywords in target's title
    let target_keywords = extract_title_keywords(&target.title);

    for keyword in target_keywords {
        let matches = same_exe
            .iter()
            .filter(|w| w.title.to_lowercase().contains(&keyword.to_lowercase()))
            .count();

        if matches == 1 {
            return Some(keyword);
        }
    }

    None
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_windows() -> Vec<WindowInfo> {
        vec![
            WindowInfo {
                hwnd: "0x1234".to_string(),
                title: "Altium Designer - PCB1.PcbDoc".to_string(),
                executable: "C:\\Program Files\\Altium\\Altium.exe".to_string(),
                pid: 1000,
                class_name: Some("TfrmAltium".to_string()),
            },
            WindowInfo {
                hwnd: "0x5678".to_string(),
                title: "Altium Designer - Schematic1.SchDoc".to_string(),
                executable: "C:\\Program Files\\Altium\\Altium.exe".to_string(),
                pid: 1000,
                class_name: Some("TfrmAltium".to_string()),
            },
            WindowInfo {
                hwnd: "0x9ABC".to_string(),
                title: "Untitled - Notepad".to_string(),
                executable: "C:\\Windows\\notepad.exe".to_string(),
                pid: 2000,
                class_name: Some("Notepad".to_string()),
            },
        ]
    }

    #[test]
    fn test_suggest_unique_exe() {
        let windows = make_windows();
        let suggestions = suggest_queries("0x9ABC", &windows).unwrap();

        // Notepad is unique exe
        assert!(suggestions.unique.iter().any(|q| q == "notepad"));
    }

    #[test]
    fn test_suggest_shared_exe() {
        let windows = make_windows();
        let suggestions = suggest_queries("0x1234", &windows).unwrap();

        // Altium is shared
        assert!(suggestions.shared.iter().any(|(q, _)| q == "altium"));

        // But with title it's unique
        assert!(suggestions
            .unique
            .iter()
            .any(|q| q.contains("title:") && q.contains("PCB")));
    }

    #[test]
    fn test_extract_exe_name() {
        assert_eq!(
            extract_exe_name("C:\\Program Files\\App\\notepad.exe"),
            "notepad"
        );
        assert_eq!(extract_exe_name("Altium.EXE"), "altium");
        assert_eq!(extract_exe_name("foo"), "foo");
    }
}
