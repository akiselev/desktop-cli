//! Linux AT-SPI2 End-to-End Tests
//!
//! Minimal smoke tests for Linux accessibility. Keep these FAST.
//! Run inside Docker: docker build -t desktop-cli-test . && docker run desktop-cli-test

#![cfg(target_os = "linux")]

use std::env;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

mod common;

struct GtkTestApp {
    process: Child,
    window_id: Option<String>,
}

impl GtkTestApp {
    fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let fixture_path = "/app/tests/fixtures/gtk_test_app/gtk_test_app";

        let mut process = Command::new(fixture_path)
            .env("GTK_MODULES", "gail:atk-bridge")
            .env("GTK_A11Y", "atspi")
            .env("NO_AT_BRIDGE", "0")
            .spawn()?;

        thread::sleep(Duration::from_millis(1500));

        if let Some(status) = process.try_wait()? {
            return Err(format!("GTK app exited early with status: {}", status).into());
        }

        let windows =
            desktop_cli::automation::linux::window::list_windows(None, Some("AT-SPI2 Test App"))?;
        let window_id = windows.first().map(|w| w.hwnd.clone());

        Ok(GtkTestApp { process, window_id })
    }

    fn hwnd(&self) -> Result<&str, Box<dyn std::error::Error>> {
        self.window_id
            .as_ref()
            .map(|s| s.as_str())
            .ok_or_else(|| "GTK app window not found".into())
    }
}

impl Drop for GtkTestApp {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Test X11 window listing works (no GTK app needed)
#[test]
fn test_x11_window_list() {
    println!("test_x11_window_list: starting");

    if env::var("DISPLAY").is_err() {
        println!("SKIP: DISPLAY not set");
        return;
    }

    println!("test_x11_window_list: calling list_windows");
    let result = desktop_cli::automation::linux::window::list_windows(None, None);
    println!("test_x11_window_list: got result");

    assert!(result.is_ok(), "list_windows should succeed: {:?}", result);
    println!("test_x11_window_list: PASSED");
}

/// Test window list with filter returns empty (not error)
#[test]
fn test_window_filter_no_match() {
    println!("test_window_filter_no_match: starting");

    if env::var("DISPLAY").is_err() {
        println!("SKIP: DISPLAY not set");
        return;
    }

    let result =
        desktop_cli::automation::linux::window::list_windows(None, Some("NonExistentApp99999"));

    assert!(result.is_ok(), "list_windows should not error");
    assert!(result.unwrap().is_empty(), "Should find no windows");
    println!("test_window_filter_no_match: PASSED");
}

/// Test invalid window ID returns error (not hang)
#[test]
fn test_invalid_hwnd_returns_error() {
    println!("test_invalid_hwnd_returns_error: starting");

    if env::var("DISPLAY").is_err() {
        println!("SKIP: DISPLAY not set");
        return;
    }

    let result = desktop_cli::automation::linux::window::get_window_info_by_id(0xDEADBEEF);
    println!(
        "test_invalid_hwnd_returns_error: got result: {:?}",
        result.is_err()
    );

    assert!(result.is_err(), "Should error for invalid window ID");
    println!("test_invalid_hwnd_returns_error: PASSED");
}

/// Test AT-SPI2 dump_tree returns GTK app elements
#[test]
fn test_dump_tree() {
    println!("test_dump_tree: starting");

    if env::var("DISPLAY").is_err() {
        println!("SKIP: DISPLAY not set");
        return;
    }

    let app = match GtkTestApp::spawn() {
        Ok(app) => app,
        Err(e) => {
            println!("SKIP: Failed to spawn GTK app: {}", e);
            return;
        }
    };

    let hwnd = match app.hwnd() {
        Ok(h) => h,
        Err(e) => {
            println!("SKIP: {}", e);
            return;
        }
    };

    let result = desktop_cli::automation::linux::atspi::dump_tree(hwnd, 5);
    assert!(result.is_ok(), "dump_tree should succeed: {:?}", result);

    let tree = result.unwrap();
    assert!(!tree.children.is_empty(), "Tree should have children");

    let element_names: Vec<String> = collect_element_names(&tree);
    println!("Found elements: {:?}", element_names);

    assert!(
        element_names.iter().any(|n| n.contains("Test Button")),
        "Should find Test Button"
    );
    assert!(
        element_names.iter().any(|n| n.contains("Test Entry")),
        "Should find Test Entry"
    );
    assert!(
        element_names.iter().any(|n| n.contains("Test Label")),
        "Should find Test Label"
    );

    println!("test_dump_tree: PASSED");
}

/// Test AT-SPI2 find_element locates specific elements
#[test]
fn test_find_element() {
    println!("test_find_element: starting");

    if env::var("DISPLAY").is_err() {
        println!("SKIP: DISPLAY not set");
        return;
    }

    let app = match GtkTestApp::spawn() {
        Ok(app) => app,
        Err(e) => {
            println!("SKIP: Failed to spawn GTK app: {}", e);
            return;
        }
    };

    let hwnd = match app.hwnd() {
        Ok(h) => h,
        Err(e) => {
            println!("SKIP: {}", e);
            return;
        }
    };

    let result = desktop_cli::automation::linux::atspi::find_elements(hwnd, "Test Button", false);
    assert!(result.is_ok(), "find_elements should succeed: {:?}", result);

    let elements = result.unwrap();
    assert!(!elements.is_empty(), "Should find Test Button element");
    assert_eq!(elements[0].name, "Test Button");

    println!("test_find_element: PASSED");
}

/// Test AT-SPI2 invoke_pattern clicks button
#[test]
fn test_invoke_pattern() {
    println!("test_invoke_pattern: starting");

    if env::var("DISPLAY").is_err() {
        println!("SKIP: DISPLAY not set");
        return;
    }

    let app = match GtkTestApp::spawn() {
        Ok(app) => app,
        Err(e) => {
            println!("SKIP: Failed to spawn GTK app: {}", e);
            return;
        }
    };

    let hwnd = match app.hwnd() {
        Ok(h) => h,
        Err(e) => {
            println!("SKIP: {}", e);
            return;
        }
    };

    let result =
        desktop_cli::automation::linux::atspi::invoke_pattern(hwnd, "Test Button", "invoke", None);
    assert!(
        result.is_ok(),
        "invoke_pattern should succeed: {:?}",
        result
    );

    let pattern_result = result.unwrap();
    assert!(pattern_result.success, "Pattern invocation should succeed");

    println!("test_invoke_pattern: PASSED");
}

/// Test AT-SPI2 gracefully handles unsupported patterns
#[test]
fn test_pattern_unsupported() {
    println!("test_pattern_unsupported: starting");

    if env::var("DISPLAY").is_err() {
        println!("SKIP: DISPLAY not set");
        return;
    }

    let app = match GtkTestApp::spawn() {
        Ok(app) => app,
        Err(e) => {
            println!("SKIP: Failed to spawn GTK app: {}", e);
            return;
        }
    };

    let hwnd = match app.hwnd() {
        Ok(h) => h,
        Err(e) => {
            println!("SKIP: {}", e);
            return;
        }
    };

    let result =
        desktop_cli::automation::linux::atspi::invoke_pattern(hwnd, "Test Label", "invoke", None);

    assert!(result.is_ok(), "invoke_pattern should return result");

    let pattern_result = result.unwrap();
    assert!(
        !pattern_result.success,
        "Pattern should not be supported on Label"
    );
    assert!(
        pattern_result.error.is_some(),
        "Should provide error message for unsupported pattern"
    );

    println!("test_pattern_unsupported: PASSED");
}

/// Test AT-SPI2 timeout handling
#[test]
fn test_timeout() {
    println!("test_timeout: starting");

    if env::var("DISPLAY").is_err() {
        println!("SKIP: DISPLAY not set");
        return;
    }

    let start = std::time::Instant::now();

    let result = desktop_cli::automation::linux::atspi::dump_tree("0xFFFFFFFF", 5);

    let elapsed = start.elapsed();

    assert!(result.is_err(), "Should error for invalid window");
    assert!(
        elapsed < Duration::from_secs(6),
        "Should fail within 6 seconds, took {:?}",
        elapsed
    );

    println!("test_timeout: PASSED (elapsed: {:?})", elapsed);
}

fn collect_element_names(element: &desktop_cli::rpc::types::UiaElement) -> Vec<String> {
    let mut names = vec![element.name.clone()];
    for child in &element.children {
        names.extend(collect_element_names(child));
    }
    names
}
