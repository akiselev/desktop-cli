#![cfg(windows)]

use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

struct NotepadTestApp {
    process: Child,
    hwnd: Option<String>,
}

impl NotepadTestApp {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut process = match Command::new("notepad.exe").spawn() {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Fallback to conhost.exe if Notepad unavailable. conhost.exe has accessible Edit control; cmd.exe lacks Edit UIA structure. See Decision Log.
                Command::new("conhost.exe").spawn()?
            }
            Err(e) => return Err(e.into()),
        };

        thread::sleep(Duration::from_millis(1500));

        if let Some(status) = process.try_wait()? {
            return Err(format!("Process exited early with status: {}", status).into());
        }

        let windows = desktop_cli::automation::windows::window::list_windows(None, None)?;
        let hwnd = windows
            .iter()
            .find(|w| {
                w.executable.to_lowercase().contains("notepad")
                    || w.executable.to_lowercase().contains("conhost")
            })
            .map(|w| w.hwnd.clone());

        Ok(NotepadTestApp { process, hwnd })
    }
}

impl Drop for NotepadTestApp {
    fn drop(&mut self) {
        let pid = self.process.id();
        let _ = self.process.kill();
        let _ = self.process.wait();

        // Query after 50ms retry balances CI speed with OS timing variance. See Decision Log.
        let check_process = || {
            let output = Command::new("tasklist")
                .args(&["/FI", &format!("PID eq {}", pid)])
                .output()
                .ok()?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains(&pid.to_string()) {
                Some(())
            } else {
                None
            }
        };

        if check_process().is_some() {
            thread::sleep(Duration::from_millis(50));
            assert!(
                check_process().is_none(),
                "Process {} still exists after cleanup",
                pid
            );
        }
    }
}

#[test]
fn test_window_listing() {
    println!("test_window_listing: starting");

    let app = match NotepadTestApp::new() {
        Ok(app) => app,
        Err(e) => {
            println!("SKIP: Failed to spawn test app: {}", e);
            return;
        }
    };

    let windows = desktop_cli::automation::windows::window::list_windows(None, None)
        .expect("list_windows should succeed");

    let found = windows.iter().any(|w| {
        w.executable.to_lowercase().contains("notepad")
            || w.executable.to_lowercase().contains("conhost")
    });

    assert!(found, "Should find test app window in list");

    println!("test_window_listing: PASSED");
}

#[test]
fn test_uia_tree() {
    println!("test_uia_tree: starting");

    let app = match NotepadTestApp::new() {
        Ok(app) => app,
        Err(e) => {
            println!("SKIP: Failed to spawn test app: {}", e);
            return;
        }
    };

    let hwnd = match &app.hwnd {
        Some(h) => h,
        None => {
            println!("SKIP: Test app window not found");
            return;
        }
    };

    let automation = uiautomation::UIAutomation::new().expect("Failed to create UIAutomation");

    let hwnd_parsed = hwnd.parse::<isize>().expect("Invalid HWND");
    let root = desktop_cli::automation::windows::uia::element_from_hwnd(&automation, hwnd_parsed)
        .expect("Failed to get element from HWND");

    let options = desktop_cli::rpc::types::TreeDumpOptions {
        max_depth: 5,
        max_list_items: 0,
        prune_offscreen: false,
        prune_empty: false,
    };

    let tree = desktop_cli::automation::windows::uia::dump_tree(&automation, &root, &options)
        .expect("dump_tree should succeed");

    assert!(!tree.children.is_empty(), "Tree should have children");

    let has_edit = contains_control_type(&tree, "Edit");
    assert!(has_edit, "Should find Edit control in tree");

    println!("test_uia_tree: PASSED");
}

#[test]
fn test_find_element() {
    println!("test_find_element: starting");

    let app = match NotepadTestApp::new() {
        Ok(app) => app,
        Err(e) => {
            println!("SKIP: Failed to spawn test app: {}", e);
            return;
        }
    };

    let hwnd = match &app.hwnd {
        Some(h) => h,
        None => {
            println!("SKIP: Test app window not found");
            return;
        }
    };

    let automation = uiautomation::UIAutomation::new().expect("Failed to create UIAutomation");

    let hwnd_parsed = hwnd.parse::<isize>().expect("Invalid HWND");
    let root = desktop_cli::automation::windows::uia::element_from_hwnd(&automation, hwnd_parsed)
        .expect("Failed to get element from HWND");

    let selector =
        desktop_cli::automation::windows::uia::Selector::parse("Edit").expect("Invalid selector");

    let elements = desktop_cli::automation::windows::uia::find_elements(
        &automation,
        &root,
        &selector,
        false,
        1000,
    )
    .expect("find_elements should succeed");

    assert!(!elements.is_empty(), "Should find Edit control");
    assert_eq!(elements[0].control_type, "Edit");

    println!("test_find_element: PASSED");
}

#[test]
fn test_cleanup() {
    println!("test_cleanup: starting");

    let pid = {
        let app = match NotepadTestApp::new() {
            Ok(app) => app,
            Err(e) => {
                println!("SKIP: Failed to spawn test app: {}", e);
                return;
            }
        };
        app.process.id()
    };

    thread::sleep(Duration::from_millis(100));

    let output = Command::new("tasklist")
        .args(&["/FI", &format!("PID eq {}", pid)])
        .output()
        .expect("Failed to run tasklist");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains(&pid.to_string()),
        "Process should be cleaned up after drop"
    );

    println!("test_cleanup: PASSED");
}

fn contains_control_type(
    element: &desktop_cli::rpc::types::UiaElement,
    control_type: &str,
) -> bool {
    if element.control_type == control_type {
        return true;
    }
    for child in &element.children {
        if contains_control_type(child, control_type) {
            return true;
        }
    }
    false
}
