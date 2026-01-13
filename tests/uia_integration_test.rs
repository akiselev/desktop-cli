use desktop_cli::automation::windows::uia::tree::{dump_tree, element_from_hwnd, element_to_uia};
use desktop_cli::automation::windows::window::list_windows;
use desktop_cli::rpc::types::TreeDumpOptions;
use uiautomation::UIAutomation;

#[test]
#[cfg(windows)]
fn test_uia_initialization() {
    // Test that we can initialize UIAutomation
    let automation = UIAutomation::new();
    assert!(automation.is_ok(), "Failed to initialize UIAutomation");
}

#[test]
#[cfg(windows)]
fn test_list_windows() {
    // Test that we can list windows
    let windows = list_windows(None, None);
    assert!(windows.is_ok(), "Failed to list windows");

    let windows = windows.unwrap();
    println!("Found {} windows", windows.len());

    // Should find at least one window (the test runner or system windows)
    assert!(windows.len() > 0, "Should find at least one window");

    // Print first few windows for debugging
    for (i, window) in windows.iter().take(5).enumerate() {
        println!("Window {}: title='{}', exe='{}'", i, window.title, window.executable);
    }
}

#[test]
#[cfg(windows)]
fn test_element_from_hwnd() {
    // Get a window to test with
    let windows = list_windows(None, None).expect("Failed to list windows");
    if windows.is_empty() {
        println!("No windows found, skipping test");
        return;
    }

    let window = &windows[0];
    let hwnd_str = &window.hwnd;
    let hwnd = hwnd_str.parse::<isize>().expect("Failed to parse HWND");

    let automation = UIAutomation::new().expect("Failed to initialize UIAutomation");
    let element = element_from_hwnd(&automation, hwnd);

    assert!(element.is_ok(), "Failed to get element from HWND: {:?}", element.err());

    let element = element.unwrap();
    let name = element.get_name().unwrap_or_default();
    println!("Element name: {}", name);
}

#[test]
#[cfg(windows)]
fn test_dump_tree() {
    // Get a window to test with
    let windows = list_windows(None, None).expect("Failed to list windows");
    if windows.is_empty() {
        println!("No windows found, skipping test");
        return;
    }

    let window = &windows[0];
    let hwnd_str = &window.hwnd;
    let hwnd = hwnd_str.parse::<isize>().expect("Failed to parse HWND");

    let automation = UIAutomation::new().expect("Failed to initialize UIAutomation");
    let root = element_from_hwnd(&automation, hwnd).expect("Failed to get root element");

    let options = TreeDumpOptions {
        max_depth: 2,  // Small depth for testing
        prune_offscreen: true,
        prune_empty: true,
        max_list_items: 5,
    };

    let tree = dump_tree(&automation, &root, &options);
    assert!(tree.is_ok(), "Failed to dump tree: {:?}", tree.err());

    let tree = tree.unwrap();
    println!("Root element: {}", tree.name);
    println!("Control type: {}", tree.control_type);
    println!("Number of children: {}", tree.children.len());
    println!("Patterns: {:?}", tree.patterns);

    // Should have some basic properties
    assert!(!tree.control_type.is_empty(), "Control type should not be empty");
}

#[test]
#[cfg(windows)]
fn test_element_to_uia_conversion() {
    // Get a window to test with
    let windows = list_windows(None, None).expect("Failed to list windows");
    if windows.is_empty() {
        println!("No windows found, skipping test");
        return;
    }

    let window = &windows[0];
    let hwnd_str = &window.hwnd;
    let hwnd = hwnd_str.parse::<isize>().expect("Failed to parse HWND");

    let automation = UIAutomation::new().expect("Failed to initialize UIAutomation");
    let element = element_from_hwnd(&automation, hwnd).expect("Failed to get element");

    // Convert to our UiaElement type
    let uia_element = element_to_uia(&element, 0);

    println!("UIA Element:");
    println!("  Name: {}", uia_element.name);
    println!("  Control Type: {}", uia_element.control_type);
    println!("  Automation ID: {}", uia_element.automation_id);
    println!("  Class Name: {}", uia_element.class_name);
    println!("  Patterns: {:?}", uia_element.patterns);
    println!("  Bounds: {:?}", uia_element.bounds);
    println!("  Enabled: {}", uia_element.is_enabled);
    println!("  Offscreen: {}", uia_element.is_offscreen);

    // Basic validation
    assert!(!uia_element.control_type.is_empty(), "Should have a control type");
}
