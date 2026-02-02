use desktop_cli::automation::types::{WindowInfo, WindowRect};

/// Creates a mock WindowInfo struct for testing
#[allow(dead_code)]
pub fn mock_window_info(hwnd: &str, title: &str, executable: &str, pid: u32) -> WindowInfo {
    WindowInfo {
        hwnd: hwnd.to_string(),
        title: title.to_string(),
        executable: executable.to_string(),
        rect: WindowRect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        },
        pid,
        class_name: Some("TestWindow".to_string()),
    }
}

/// Generates a list of mock windows for testing
#[allow(dead_code)]
pub fn generate_mock_windows() -> Vec<WindowInfo> {
    vec![
        mock_window_info(
            "0x1001",
            "Firefox - Mozilla Firefox",
            "/usr/bin/firefox",
            1234,
        ),
        mock_window_info("0x1002", "Terminal", "/usr/bin/gnome-terminal", 1235),
        mock_window_info("0x1003", "Visual Studio Code", "/usr/bin/code", 1236),
        mock_window_info(
            "0x1004",
            "Notepad",
            "C:\\Windows\\System32\\notepad.exe",
            1237,
        ),
        mock_window_info(
            "0x1005",
            "Chrome Browser",
            "/opt/google/chrome/chrome",
            1238,
        ),
    ]
}
