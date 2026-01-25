//! Linux AT-SPI2 End-to-End Tests
//!
//! Minimal smoke tests for Linux accessibility. Keep these FAST.
//! Run inside Docker: docker build -t desktop-cli-test . && docker run desktop-cli-test

#![cfg(target_os = "linux")]

use std::env;

mod common;

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

    // This should return an error quickly, not hang
    let result = desktop_cli::automation::linux::window::get_window_info_by_id(0xDEADBEEF);
    println!(
        "test_invalid_hwnd_returns_error: got result: {:?}",
        result.is_err()
    );

    // We expect an error for non-existent window
    assert!(result.is_err(), "Should error for invalid window ID");
    println!("test_invalid_hwnd_returns_error: PASSED");
}
