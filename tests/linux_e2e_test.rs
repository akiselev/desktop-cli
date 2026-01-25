//! Linux AT-SPI2 End-to-End Tests
//!
//! Tests the Linux accessibility automation against a real GTK test application.
//! Requires: Xvfb, D-Bus session, AT-SPI2 service, GTK test app built.
//!
//! Run inside Docker: docker build -t desktop-cli-test . && docker run desktop-cli-test

#![cfg(target_os = "linux")]

use std::env;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

mod common;

/// Path to the GTK test application binary
const GTK_TEST_APP_PATH: &str = "tests/fixtures/gtk_test_app/gtk_test_app";

/// Spawn the GTK test application with proper accessibility environment
fn spawn_gtk_test_app() -> Option<Child> {
    let display = env::var("DISPLAY").ok()?;

    Command::new(GTK_TEST_APP_PATH)
        .env("DISPLAY", display)
        .env("GTK_MODULES", "gail:atk-bridge")
        .env("GTK_A11Y", "atspi")
        .env("NO_AT_BRIDGE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

/// Wait for a window with the given title to appear in the window list
fn wait_for_window(title: &str, timeout: Duration) -> Option<String> {
    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    while start.elapsed() < timeout {
        if let Ok(windows) = desktop_cli::automation::linux::window::list_windows(None, Some(title))
        {
            if let Some(window) = windows.first() {
                return Some(window.hwnd.clone());
            }
        }
        thread::sleep(poll_interval);
    }
    None
}

/// Verify the GTK test app has all expected accessible elements
fn verify_gtk_app_elements(hwnd: &str) -> Result<(), String> {
    let tree = desktop_cli::automation::linux::atspi::dump_tree(hwnd, 5)
        .map_err(|e| format!("Failed to dump tree: {}", e))?;

    // Recursively search for expected element names
    let expected = ["Test Button", "Test Entry", "Test Label"];
    let mut found = vec![false; expected.len()];

    fn search_tree(
        element: &desktop_cli::rpc::types::UiaElement,
        expected: &[&str],
        found: &mut [bool],
    ) {
        for (i, name) in expected.iter().enumerate() {
            if element.name.contains(name) {
                found[i] = true;
            }
        }
        for child in &element.children {
            search_tree(child, expected, found);
        }
    }

    search_tree(&tree, &expected, &mut found);

    for (i, name) in expected.iter().enumerate() {
        if !found[i] {
            return Err(format!(
                "Expected element '{}' not found in AT-SPI2 tree. \
                 App may be running but elements not properly accessible.",
                name
            ));
        }
    }

    Ok(())
}

/// Test that window enumeration finds the GTK test app
#[test]
fn test_window_enumeration() {
    // Check if we have a DISPLAY (Xvfb must be running)
    if env::var("DISPLAY").is_err() {
        eprintln!("Skipping test: DISPLAY not set (no X11)");
        return;
    }

    // Check if GTK test app exists
    if !std::path::Path::new(GTK_TEST_APP_PATH).exists() {
        eprintln!(
            "Skipping test: GTK test app not built at {}",
            GTK_TEST_APP_PATH
        );
        return;
    }

    // Spawn the GTK test app
    let mut child = match spawn_gtk_test_app() {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: Failed to spawn GTK test app");
            return;
        }
    };

    // Wait for window to appear (5 second timeout)
    let hwnd = wait_for_window("AT-SPI2 Test App", Duration::from_secs(5));

    // Clean up
    let _ = child.kill();
    let _ = child.wait();

    assert!(
        hwnd.is_some(),
        "GTK test app window not found in window list"
    );
}

/// Test that dump_tree returns elements from the GTK test app
#[test]
fn test_dump_tree() {
    if env::var("DISPLAY").is_err() {
        eprintln!("Skipping test: DISPLAY not set (no X11)");
        return;
    }

    if !std::path::Path::new(GTK_TEST_APP_PATH).exists() {
        eprintln!("Skipping test: GTK test app not built");
        return;
    }

    let mut child = match spawn_gtk_test_app() {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: Failed to spawn GTK test app");
            return;
        }
    };

    let hwnd = wait_for_window("AT-SPI2 Test App", Duration::from_secs(5));
    let result = if let Some(ref hwnd) = hwnd {
        desktop_cli::automation::linux::atspi::dump_tree(hwnd, 10)
    } else {
        Err(desktop_cli::error::DesktopCliError::Platform(
            "Window not found".to_string(),
        ))
    };

    let _ = child.kill();
    let _ = child.wait();

    let tree = result.expect("dump_tree should succeed");
    assert!(
        !tree.name.is_empty() || !tree.children.is_empty(),
        "Tree should have content"
    );
}

/// Test that find_element locates specific widgets
#[test]
fn test_find_element() {
    if env::var("DISPLAY").is_err() {
        eprintln!("Skipping test: DISPLAY not set (no X11)");
        return;
    }

    if !std::path::Path::new(GTK_TEST_APP_PATH).exists() {
        eprintln!("Skipping test: GTK test app not built");
        return;
    }

    let mut child = match spawn_gtk_test_app() {
        Some(c) => c,
        None => {
            eprintln!("Skipping test: Failed to spawn GTK test app");
            return;
        }
    };

    let hwnd = wait_for_window("AT-SPI2 Test App", Duration::from_secs(5));
    let result = if let Some(ref hwnd) = hwnd {
        // First verify elements are accessible
        if let Err(e) = verify_gtk_app_elements(hwnd) {
            eprintln!("Warning: {}", e);
        }

        desktop_cli::automation::linux::atspi::find_elements(hwnd, "Test Button", false)
    } else {
        Err(desktop_cli::error::DesktopCliError::Platform(
            "Window not found".to_string(),
        ))
    };

    let _ = child.kill();
    let _ = child.wait();

    let elements = result.expect("find_elements should succeed");
    assert!(
        !elements.is_empty(),
        "Should find 'Test Button' element in GTK test app"
    );
}

/// Test error handling for invalid window ID
#[test]
fn test_invalid_window_id() {
    if env::var("DISPLAY").is_err() {
        eprintln!("Skipping test: DISPLAY not set (no X11)");
        return;
    }

    let result = desktop_cli::automation::linux::atspi::dump_tree("0xdeadbeef", 5);
    assert!(
        result.is_err(),
        "dump_tree should fail for non-existent window"
    );
}

/// Test graceful handling when app is not running
#[test]
fn test_app_not_running() {
    if env::var("DISPLAY").is_err() {
        eprintln!("Skipping test: DISPLAY not set (no X11)");
        return;
    }

    // Try to find a window that doesn't exist
    let windows =
        desktop_cli::automation::linux::window::list_windows(None, Some("NonExistentApp12345"));

    assert!(
        windows.is_ok(),
        "list_windows should not error for non-matching filter"
    );
    assert!(
        windows.unwrap().is_empty(),
        "Should find no windows for non-existent app"
    );
}
