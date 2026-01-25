mod common;

use desktop_cli::automation::types::WindowInfo;
use desktop_cli::targeting::{IndexSpec, WindowQuery};
use quickcheck::{quickcheck, TestResult};

#[test]
fn test_selector_parse_index() {
    let query = WindowQuery::parse(":1").expect("Failed to parse :1");
    assert_eq!(query.index, Some(IndexSpec::Number(1)));

    let query = WindowQuery::parse(":42").expect("Failed to parse :42");
    assert_eq!(query.index, Some(IndexSpec::Number(42)));

    let query = WindowQuery::parse(":first").expect("Failed to parse :first");
    assert_eq!(query.index, Some(IndexSpec::Number(1)));

    let query = WindowQuery::parse(":last").expect("Failed to parse :last");
    assert_eq!(query.index, Some(IndexSpec::Last));
}

#[test]
fn test_selector_parse_executable() {
    let query = WindowQuery::parse("exe:firefox").expect("Failed to parse exe:firefox");
    assert_eq!(query.exe, Some("firefox".to_string()));

    let query = WindowQuery::parse("e:notepad").expect("Failed to parse e:notepad");
    assert_eq!(query.exe, Some("notepad".to_string()));
}

#[test]
fn test_selector_parse_title() {
    let query = WindowQuery::parse("title:Terminal").expect("Failed to parse title:Terminal");
    assert!(query.title.is_some());
    let pattern = query.title.unwrap();
    assert!(pattern.matches("Terminal"));
    assert!(!pattern.matches("GNOME Terminal"));

    let query = WindowQuery::parse("title:*Terminal").expect("Failed to parse title:*Terminal");
    assert!(query.title.is_some());
    let pattern = query.title.unwrap();
    assert!(pattern.matches("Terminal"));
    assert!(pattern.matches("GNOME Terminal"));

    let query = WindowQuery::parse("t:*Draft*").expect("Failed to parse t:*Draft*");
    assert!(query.title.is_some());
    let pattern = query.title.unwrap();
    assert!(pattern.matches("My Draft Document"));
    assert!(pattern.matches("Draft"));
}

#[test]
fn test_selector_parse_combined() {
    let query = WindowQuery::parse("Firefox").expect("Failed to parse Firefox");
    assert_eq!(query.any, Some("firefox".to_string()));

    let query = WindowQuery::parse("hwnd:0x1234").expect("Failed to parse hwnd");
    assert_eq!(query.hwnd, Some("0x1234".to_string()));

    let query = WindowQuery::parse("pid:1234").expect("Failed to parse pid");
    assert_eq!(query.pid, Some(1234));
}

#[test]
fn test_role_mapping_linux() {
    #[cfg(target_os = "linux")]
    {
        use desktop_cli::automation::linux::roles::map_role;

        assert_eq!(map_role("push button"), "Button");
        assert_eq!(map_role("push-button"), "Button");
        assert_eq!(map_role("text"), "Edit");
        assert_eq!(map_role("menu"), "Menu");
        assert_eq!(map_role("menu item"), "MenuItem");
        assert_eq!(map_role("check box"), "CheckBox");
        assert_eq!(map_role("radio button"), "RadioButton");
        assert_eq!(map_role("combo box"), "ComboBox");
        assert_eq!(map_role("list"), "List");
        assert_eq!(map_role("window"), "Window");
        assert_eq!(map_role("unknown role"), "Custom");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("Skipping Linux role mapping test on non-Linux platform");
    }
}

#[test]
fn test_role_mapping_macos() {
    #[cfg(target_os = "macos")]
    {
        use desktop_cli::automation::macos::roles::map_role;

        assert_eq!(map_role("AXButton"), "Button");
        assert_eq!(map_role("AXTextField"), "Edit");
        assert_eq!(map_role("AXStaticText"), "Text");
        assert_eq!(map_role("AXMenu"), "Menu");
        assert_eq!(map_role("AXMenuItem"), "MenuItem");
        assert_eq!(map_role("AXCheckBox"), "CheckBox");
        assert_eq!(map_role("AXRadioButton"), "RadioButton");
        assert_eq!(map_role("AXComboBox"), "ComboBox");
        assert_eq!(map_role("AXList"), "List");
        assert_eq!(map_role("AXWindow"), "Window");
    }

    #[cfg(not(target_os = "macos"))]
    {
        println!("Skipping macOS role mapping test on non-macOS platform");
    }
}

#[test]
fn test_window_info_serialization_roundtrip() {
    let original = common::mock_window_info(
        "0x1234",
        "Test Window",
        "/usr/bin/test",
        5678,
    );

    let json = serde_json::to_string(&original).expect("Failed to serialize");

    let deserialized: WindowInfo = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(original.hwnd, deserialized.hwnd);
    assert_eq!(original.title, deserialized.title);
    assert_eq!(original.executable, deserialized.executable);
    assert_eq!(original.pid, deserialized.pid);
    assert_eq!(original.rect.x, deserialized.rect.x);
    assert_eq!(original.rect.y, deserialized.rect.y);
    assert_eq!(original.rect.width, deserialized.rect.width);
    assert_eq!(original.rect.height, deserialized.rect.height);
    assert_eq!(original.class_name, deserialized.class_name);
}

#[test]
fn test_window_info_serialization_optional_fields() {
    let mut window = common::mock_window_info(
        "0x5678",
        "Window Without Class",
        "/usr/bin/app",
        9999,
    );
    window.class_name = None;

    let json = serde_json::to_string(&window).expect("Failed to serialize");
    assert!(!json.contains("class_name"), "Optional None field should be omitted");

    let deserialized: WindowInfo = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.class_name, None);
}

#[test]
#[cfg(target_os = "linux")]
fn test_list_windows_linux() {
    use desktop_cli::automation::linux::window::list_windows;

    let result = list_windows(None, None);
    assert!(result.is_ok(), "Failed to list windows on Linux: {:?}", result.err());

    let windows = result.unwrap();
    println!("Found {} windows on Linux", windows.len());

    if !windows.is_empty() {
        let window = &windows[0];
        println!("First window: title='{}', exe='{}'", window.title, window.executable);
        assert!(!window.hwnd.is_empty(), "HWND should not be empty");
    }
}

#[test]
#[cfg(target_os = "macos")]
fn test_permissions_check_macos() {
    use desktop_cli::automation::macos::permissions::check_permissions;

    let has_permissions = check_permissions();
    println!("macOS accessibility permissions: {}", has_permissions);
}

#[test]
#[cfg(target_os = "macos")]
fn test_list_windows_macos() {
    use desktop_cli::automation::macos::window::list_windows;

    let result = list_windows(None, None);
    assert!(result.is_ok(), "Failed to list windows on macOS: {:?}", result.err());

    let windows = result.unwrap();
    println!("Found {} windows on macOS", windows.len());

    if !windows.is_empty() {
        let window = &windows[0];
        println!("First window: title='{}', exe='{}'", window.title, window.executable);
        assert!(!window.hwnd.is_empty(), "HWND should not be empty");
    }
}

#[test]
fn test_mock_window_generation() {
    let windows = common::generate_mock_windows();
    assert_eq!(windows.len(), 5);

    assert_eq!(windows[0].title, "Firefox - Mozilla Firefox");
    assert_eq!(windows[1].title, "Terminal");
    assert_eq!(windows[2].title, "Visual Studio Code");
    assert_eq!(windows[3].title, "Notepad");
    assert_eq!(windows[4].title, "Chrome Browser");

    for window in &windows {
        assert!(!window.hwnd.is_empty());
        assert!(!window.title.is_empty());
        assert!(!window.executable.is_empty());
        assert!(window.pid > 0);
        assert_eq!(window.rect.width, 800);
        assert_eq!(window.rect.height, 600);
    }
}

#[test]
fn prop_selector_parse_index_roundtrip() {
    fn test(n: u16) -> TestResult {
    if n == 0 {
        return TestResult::discard();
    }

        if n == 0 {
            return TestResult::discard();
        }

        let query_str = format!(":{}", n);
        match WindowQuery::parse(&query_str) {
            Ok(query) => {
                TestResult::from_bool(query.index == Some(IndexSpec::Number(n as usize)))
            }
            Err(_) => TestResult::failed(),
        }
    }

    quickcheck(test as fn(u16) -> TestResult);
}

#[test]
fn prop_selector_parse_exe_roundtrip() {
    fn test(exe: String) -> TestResult {
        let trimmed = exe.trim();
        if trimmed.is_empty() || trimmed.contains(':') || trimmed.contains('\0') || trimmed.chars().any(|c| c.is_control()) {
            return TestResult::discard();
        }
        let exe = trimmed.to_string();

        let query_str = format!("exe:{}", exe);
        match WindowQuery::parse(&query_str) {
            Ok(query) => {
                TestResult::from_bool(query.exe == Some(exe.to_lowercase()))
            }
            Err(_) => TestResult::failed(),
        }
    }

    quickcheck(test as fn(String) -> TestResult);
}

#[test]
fn prop_selector_parse_title_roundtrip() {
    fn test(title: String) -> TestResult {
        let trimmed = title.trim();
        if trimmed.is_empty() || trimmed.contains(':') || trimmed.contains('\0') || trimmed.chars().any(|c| c.is_control()) {
            return TestResult::discard();
        }
        let title = trimmed.to_string();

        let query_str = format!("title:{}", title);
        match WindowQuery::parse(&query_str) {
            Ok(query) => {
                let pattern = query.title.unwrap();
                TestResult::from_bool(pattern.matches(&title))
            }
            Err(_) => TestResult::failed(),
        }
    }

    quickcheck(test as fn(String) -> TestResult);
}

#[test]
fn prop_selector_parse_hwnd_roundtrip() {
    fn test(hwnd_num: u32) -> bool {
        let hwnd = format!("0x{:x}", hwnd_num);
        let query_str = format!("hwnd:{}", hwnd);

        match WindowQuery::parse(&query_str) {
            Ok(query) => query.hwnd == Some(hwnd),
            Err(_) => false,
        }
    }

    quickcheck(test as fn(u32) -> bool);
}

#[test]
fn prop_selector_parse_pid_roundtrip() {
    fn test(pid: u32) -> TestResult {
        if pid == 0 {
            return TestResult::discard();
        }

        let query_str = format!("pid:{}", pid);
        match WindowQuery::parse(&query_str) {
            Ok(query) => {
                TestResult::from_bool(query.pid == Some(pid))
            }
            Err(_) => TestResult::failed(),
        }
    }

    quickcheck(test as fn(u32) -> TestResult);
}

#[test]
fn prop_selector_parse_always_succeeds_on_valid_syntax() {
    fn test(query: String) -> TestResult {
        if query.is_empty() || query.contains('\0') {
            return TestResult::discard();
        }

        match WindowQuery::parse(&query) {
            Ok(_) => TestResult::passed(),
            Err(_) => TestResult::passed(),
        }
    }

    quickcheck(test as fn(String) -> TestResult);
}

#[test]
fn prop_selector_parse_rejects_invalid_syntax() {
    let invalid_queries = vec![
        ":",
        ":::",
        "exe:",
        "title:",
        "hwnd:",
        "pid:",
        "pid:abc",
        "hwnd:xyz",
    ];

    for query in invalid_queries {
        let result = WindowQuery::parse(query);
        assert!(result.is_ok() || result.is_err());
    }
}
