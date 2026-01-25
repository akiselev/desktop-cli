//! AT-SPI2 accessibility tree operations

use super::window::parse_window_id;
use crate::automation::linux::roles::map_role;
use crate::error::{DesktopCliError, Result};
use crate::rpc::types::{PatternResult, QueryResult, UiaElement};
use atspi::proxy::accessible::AccessibleProxy;
use atspi::proxy::action::ActionProxy;
use atspi::proxy::component::ComponentProxy;
use atspi::proxy::text::TextProxy;
use atspi::proxy::value::ValueProxy;
use atspi::{AccessibilityConnection, CoordType, Interface, InterfaceSet, Role};
use std::time::Duration;
use zbus::fdo::DBusProxy;

/// Connect to AT-SPI2 and get accessible window for an application matching the X11 window's PID
///
/// Uses PID matching: gets the X11 window's _NET_WM_PID, then finds the AT-SPI2 application
/// whose D-Bus connection has the same PID via org.freedesktop.DBus.GetConnectionUnixProcessID.
async fn get_accessible_for_window(
    window_id: u32,
) -> Result<(AccessibilityConnection, String, String)> {
    // Get the X11 window info including PID
    let window_info = crate::automation::linux::window::get_window_info_by_id(window_id)?;
    let target_pid = window_info.pid;

    if target_pid == 0 {
        return Err(DesktopCliError::Platform(format!(
            "Window 0x{:x} has no _NET_WM_PID property. Cannot match to AT-SPI2 application.",
            window_id
        )));
    }

    let connection = AccessibilityConnection::new().await.map_err(|e| {
        DesktopCliError::Platform(format!(
            "Failed to connect to AT-SPI2 service. Is the accessibility service running? Error: {}",
            e
        ))
    })?;

    let zbus_conn = connection.connection().clone();

    // Create D-Bus proxy to query PIDs of AT-SPI2 connections
    let dbus_proxy = DBusProxy::new(&zbus_conn)
        .await
        .map_err(|e| DesktopCliError::Platform(format!("Failed to create D-Bus proxy: {}", e)))?;

    // Get all AT-SPI2 applications from registry
    let registry_accessible = AccessibleProxy::builder(&zbus_conn)
        .destination("org.a11y.atspi.Registry")
        .map_err(|e| DesktopCliError::Platform(format!("Failed to build registry proxy: {}", e)))?
        .path("/org/a11y/atspi/accessible/root")
        .map_err(|e| DesktopCliError::Platform(format!("Failed to set path: {}", e)))?
        .build()
        .await
        .map_err(|e| {
            DesktopCliError::Platform(format!("Failed to build accessible proxy: {}", e))
        })?;

    let children = registry_accessible.get_children().await.map_err(|e| {
        DesktopCliError::Platform(format!("Failed to get desktop applications: {}", e))
    })?;

    // Find the app whose D-Bus connection has the matching PID
    for child_ref in &children {
        // Get PID of this AT-SPI2 app's D-Bus connection
        let app_pid = dbus_proxy
            .get_connection_unix_process_id(child_ref.name.as_str().try_into().unwrap())
            .await
            .ok();

        if let Some(pid) = app_pid {
            if pid == target_pid {
                // Found the matching app - return its first window (or root if no windows)
                let app_builder = AccessibleProxy::builder(&zbus_conn)
                    .destination(child_ref.name.as_str())
                    .ok()
                    .and_then(|b| b.path("/org/a11y/atspi/accessible/root").ok());

                if let Some(builder) = app_builder {
                    if let Ok(acc) = builder.build().await {
                        // Try to get first window child
                        if let Ok(app_children) = acc.get_children().await {
                            if !app_children.is_empty() {
                                // Return first window
                                let window_ref = &app_children[0];
                                return Ok((
                                    connection,
                                    window_ref.name.to_string(),
                                    window_ref.path.to_string(),
                                ));
                            }
                        }
                        // No windows, return app root
                        return Ok((
                            connection,
                            child_ref.name.to_string(),
                            "/org/a11y/atspi/accessible/root".to_string(),
                        ));
                    }
                }
            }
        }
    }

    Err(DesktopCliError::Platform(format!(
        "Could not find AT-SPI2 application for window 0x{:x} (PID: {}). Make sure the application supports AT-SPI2 accessibility.",
        window_id, target_pid
    )))
}

/// Build component proxy for getting element bounds
async fn build_component_proxy<'a>(
    accessible: &AccessibleProxy<'a>,
) -> Option<(i32, i32, i32, i32)> {
    let conn = accessible.connection();
    let component = ComponentProxy::builder(conn)
        .destination(accessible.destination())
        .ok()
        .and_then(|b| b.path(accessible.path()).ok());

    if let Some(builder) = component {
        if let Ok(comp) = builder.build().await {
            comp.get_extents(CoordType::Screen).await.ok()
        } else {
            None
        }
    } else {
        None
    }
}

/// Extract element value from Value or Text interface
async fn extract_element_value<'a>(
    accessible: &AccessibleProxy<'a>,
    interfaces: &InterfaceSet,
) -> Option<String> {
    let conn = accessible.connection();
    if interfaces.contains(Interface::Value) {
        let value_builder = ValueProxy::builder(conn)
            .destination(accessible.destination())
            .ok()
            .and_then(|b| b.path(accessible.path()).ok());

        if let Some(vb) = value_builder {
            if let Ok(value_proxy) = vb.build().await {
                return value_proxy
                    .current_value()
                    .await
                    .ok()
                    .map(|v| v.to_string());
            }
        }
    } else if interfaces.contains(Interface::Text) {
        let text_builder = TextProxy::builder(conn)
            .destination(accessible.destination())
            .ok()
            .and_then(|b| b.path(accessible.path()).ok());

        if let Some(tb) = text_builder {
            if let Ok(text_proxy) = tb.build().await {
                return text_proxy.get_text(0, -1).await.ok();
            }
        }
    }
    None
}

/// Detect supported patterns from interface set
fn detect_patterns(interfaces: &InterfaceSet) -> Vec<String> {
    let mut patterns = Vec::new();
    if interfaces.contains(Interface::Action) {
        patterns.push("Invoke".to_string());
    }
    if interfaces.contains(Interface::Value) {
        patterns.push("Value".to_string());
    }
    if interfaces.contains(Interface::Text) {
        patterns.push("Text".to_string());
    }
    patterns
}

/// Execute async AT-SPI2 operation in isolated tokio runtime
///
/// Per-call runtime creation prevents state conflicts when desktop-cli
/// is used as library. Performance cost accepted for v1 simplicity.
fn with_atspi_runtime<F, T>(future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| DesktopCliError::Platform(format!("Failed to create async runtime: {}", e)))?;
    rt.block_on(future)
}

/// Traverse element tree recursively
async fn traverse_element<'a>(
    accessible: &AccessibleProxy<'a>,
    depth: u32,
    max_depth: u32,
) -> Result<UiaElement> {
    let conn = accessible.connection();
    let name = accessible.name().await.unwrap_or_default();
    let role = accessible.get_role().await.unwrap_or(Role::Unknown);
    let role_str = format!("{:?}", role);

    let (x, y, width, height) = build_component_proxy(accessible)
        .await
        .unwrap_or((0, 0, 0, 0));

    let state_set = accessible.get_state().await.unwrap_or_default();
    let is_enabled = state_set.contains(atspi::State::Enabled);
    let is_offscreen = !state_set.contains(atspi::State::Visible);

    let interfaces = accessible
        .get_interfaces()
        .await
        .unwrap_or(InterfaceSet::empty());
    let patterns = detect_patterns(&interfaces);
    let value = extract_element_value(accessible, &interfaces).await;

    let id = format!("atspi_{}_{}", accessible.path(), depth);

    let mut children = Vec::new();
    if depth < max_depth {
        if let Ok(child_refs) = accessible.get_children().await {
            for child_ref in child_refs {
                let child_builder = AccessibleProxy::builder(conn)
                    .destination(child_ref.name.clone())
                    .ok()
                    .and_then(|b| b.path(child_ref.path.clone()).ok());

                if let Some(cb) = child_builder {
                    if let Ok(child_acc) = cb.build().await {
                        if let Ok(child_elem) =
                            Box::pin(traverse_element(&child_acc, depth + 1, max_depth)).await
                        {
                            children.push(child_elem);
                        }
                    }
                }
            }
        }
    }

    Ok(UiaElement {
        id,
        control_type: map_role(&role_str.to_lowercase()),
        localized_type: role_str.clone(),
        name,
        automation_id: String::new(),
        class_name: String::new(),
        value,
        bounds: [x, y, width, height],
        is_enabled,
        is_offscreen,
        patterns,
        depth,
        children,
    })
}

/// Dump element tree from window root
pub fn dump_tree(window_id: &str, max_depth: u32) -> Result<UiaElement> {
    let window_id_num = parse_window_id(window_id)?;

    with_atspi_runtime(async {
        let (connection, dest, path) = get_accessible_for_window(window_id_num).await?;

        let zbus_conn = connection.connection();

        let accessible = AccessibleProxy::builder(zbus_conn)
            .destination(dest.as_str())
            .map_err(|e| DesktopCliError::Platform(format!("Failed to build proxy: {}", e)))?
            .path(path.as_str())
            .map_err(|e| DesktopCliError::Platform(format!("Failed to set path: {}", e)))?
            .build()
            .await
            .map_err(|e| DesktopCliError::Platform(format!("Failed to build accessible: {}", e)))?;

        traverse_element(&accessible, 0, max_depth).await
    })
}

/// Find elements matching selector
pub fn find_elements(window_id: &str, selector: &str, find_all: bool) -> Result<Vec<UiaElement>> {
    // Reduced depth from 20 to 10 for performance; AT-SPI2 tree traversal is slower than Windows UIA
    let root = dump_tree(window_id, 10)?;

    let mut results = Vec::new();
    find_matching_elements(&root, selector, find_all, &mut results);

    Ok(results)
}

/// Recursively search for elements matching selector
fn find_matching_elements(
    element: &UiaElement,
    selector: &str,
    find_all: bool,
    results: &mut Vec<UiaElement>,
) {
    if !find_all && !results.is_empty() {
        return;
    }

    if element_matches_selector(element, selector) {
        results.push(element.clone());
        if !find_all {
            return;
        }
    }

    for child in &element.children {
        find_matching_elements(child, selector, find_all, results);
        if !find_all && !results.is_empty() {
            return;
        }
    }
}

/// Check if element matches simple selector
fn element_matches_selector(element: &UiaElement, selector: &str) -> bool {
    if selector.starts_with('#') {
        let id = &selector[1..];
        element.automation_id == id || element.name.to_lowercase().contains(&id.to_lowercase())
    } else if selector.starts_with('.') {
        let class = &selector[1..];
        element.class_name == class
    } else {
        element.control_type.to_lowercase() == selector.to_lowercase()
            || element
                .name
                .to_lowercase()
                .contains(&selector.to_lowercase())
    }
}

/// Check if element matching selector exists
pub fn element_exists(window_id: &str, selector: &str) -> Result<bool> {
    let timeout = Duration::from_millis(500);

    with_atspi_runtime(async {
        match tokio::time::timeout(timeout, async { find_elements(window_id, selector, false) })
            .await
        {
            Ok(Ok(results)) => Ok(!results.is_empty()),
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(false),
        }
    })
}

/// Invoke accessibility pattern on element
pub fn invoke_pattern(
    window_id: &str,
    selector: &str,
    pattern: &str,
    action: Option<&str>,
) -> Result<PatternResult> {
    let window_id_num = parse_window_id(window_id)?;

    with_atspi_runtime(async {
        let (connection, dest, path) = get_accessible_for_window(window_id_num).await?;
        let zbus_conn = connection.connection();

        let accessible = AccessibleProxy::builder(zbus_conn)
            .destination(dest.as_str())
            .map_err(|e| DesktopCliError::Platform(format!("Failed to build proxy: {}", e)))?
            .path(path.as_str())
            .map_err(|e| DesktopCliError::Platform(format!("Failed to set path: {}", e)))?
            .build()
            .await
            .map_err(|e| DesktopCliError::Platform(format!("Failed to build accessible: {}", e)))?;

        let root = traverse_element(&accessible, 0, 10).await?;

        let mut results = Vec::new();
        find_matching_elements(&root, selector, false, &mut results);

        if results.is_empty() {
            return Ok(PatternResult::err(format!(
                "No element found matching: {}",
                selector
            )));
        }

        let _element = &results[0];

        match pattern.to_lowercase().as_str() {
            "invoke" => {
                let action_proxy = ActionProxy::builder(accessible.connection())
                    .destination(accessible.destination())
                    .map_err(|e| {
                        DesktopCliError::Platform(format!("Failed to build action proxy: {}", e))
                    })?
                    .path(accessible.path())
                    .map_err(|e| DesktopCliError::Platform(format!("Failed to set path: {}", e)))?
                    .build()
                    .await
                    .map_err(|e| {
                        DesktopCliError::Platform(format!("Failed to build action proxy: {}", e))
                    })?;

                action_proxy.do_action(0).await.map_err(|e| {
                    DesktopCliError::Platform(format!("Failed to invoke action: {}", e))
                })?;

                Ok(PatternResult::ok())
            }
            "value" => {
                if let Some(value_str) = action {
                    let value_proxy = ValueProxy::builder(accessible.connection())
                        .destination(accessible.destination())
                        .map_err(|e| {
                            DesktopCliError::Platform(format!("Failed to build value proxy: {}", e))
                        })?
                        .path(accessible.path())
                        .map_err(|e| {
                            DesktopCliError::Platform(format!("Failed to set path: {}", e))
                        })?
                        .build()
                        .await
                        .map_err(|e| {
                            DesktopCliError::Platform(format!("Failed to build value proxy: {}", e))
                        })?;

                    let value_f64 = value_str
                        .parse::<f64>()
                        .map_err(|e| DesktopCliError::Platform(format!("Invalid value: {}", e)))?;

                    value_proxy
                        .set_current_value(value_f64)
                        .await
                        .map_err(|e| {
                            DesktopCliError::Platform(format!("Failed to set value: {}", e))
                        })?;

                    Ok(PatternResult::ok())
                } else {
                    Ok(PatternResult::err(
                        "Value pattern requires action parameter".to_string(),
                    ))
                }
            }
            _ => Ok(PatternResult::err(format!(
                "Pattern not supported: {}",
                pattern
            ))),
        }
    })
}

/// Get visual summary of window or element
pub fn get_summary(
    window_id: &str,
    _selector: &str,
    _include_invisible: bool,
    _include_offscreen: bool,
    _bbox: Option<[i32; 4]>,
    max_depth: u32,
    _control_types: Option<Vec<String>>,
) -> Result<String> {
    let tree = dump_tree(window_id, max_depth)?;

    let summary = format_summary(&tree, 0);

    Ok(summary)
}

/// Format element tree as text summary
fn format_summary(element: &UiaElement, indent: usize) -> String {
    let mut result = String::new();
    let prefix = "  ".repeat(indent);

    result.push_str(&format!(
        "{}{} [{}]",
        prefix, element.control_type, element.name
    ));

    if let Some(ref value) = element.value {
        result.push_str(&format!(" = {}", value));
    }

    result.push_str(&format!(
        " ({},{} {}x{})\n",
        element.bounds[0], element.bounds[1], element.bounds[2], element.bounds[3]
    ));

    for child in &element.children {
        result.push_str(&format_summary(child, indent + 1));
    }

    result
}

/// Query elements with structured results
pub fn query_elements(window_id: &str, selector: &str, find_all: bool) -> Result<QueryResult> {
    let elements = find_elements(window_id, selector, find_all)?;

    let matches = elements
        .iter()
        .enumerate()
        .map(|(i, elem)| crate::rpc::types::ElementRef {
            id: format!("elem_{}", i),
            role: elem.control_type.clone(),
            label: elem.name.clone(),
            action: if elem.patterns.contains(&"Invoke".to_string()) {
                Some("click".to_string())
            } else {
                None
            },
            selector: format!("#{}", elem.name),
        })
        .collect();

    Ok(QueryResult {
        count: elements.len(),
        matches,
        suggestions: Vec::new(),
    })
}
