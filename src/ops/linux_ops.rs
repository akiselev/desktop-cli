//! Linux-specific operation implementations
//!
//! Direct implementations of desktop automation operations for Linux.

use crate::automation::linux::{
    accessibility::{
        self, dump_tree as at_dump_tree, execute_pattern, has_state, role_to_string, roles, states,
        AccessibleElement, AtSpiConnection, PatternOp,
    },
    capture_screenshot,
    coordinates::calculate_center,
    get_window_info as get_x11_window_info,
    input::{
        click_at_coords, click_at_screen_coords, double_click_at_coords,
        double_click_at_screen_coords, right_click_at_coords, right_click_at_screen_coords,
        scroll as input_scroll, send_keys as input_send_keys, type_text as input_type_text,
    },
    list_windows as list_windows_raw, parse_window_id, ScreenshotMethod,
};
use crate::automation::types::WindowInfo;
use crate::rpc::types::{ElementRef, PatternResult, QueryResult, Screenshot, TreeDumpOptions, UiaElement};
use x11::xlib::Window;

/// Error type for operations
#[derive(Debug)]
pub struct OpsError(pub String);

impl std::fmt::Display for OpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for OpsError {}

impl From<crate::error::DesktopCliError> for OpsError {
    fn from(e: crate::error::DesktopCliError) -> Self {
        OpsError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, OpsError>;

// ============================================================================
// Window Operations
// ============================================================================

/// List all visible windows with optional filters
pub fn list_windows(exe_filter: Option<&str>, title_filter: Option<&str>) -> Result<Vec<WindowInfo>> {
    list_windows_raw(exe_filter, title_filter).map_err(|e| OpsError(e.to_string()))
}

/// Get info for a specific window by window ID string
pub fn get_window_by_hwnd(window_str: &str) -> Result<WindowInfo> {
    let window = parse_window_id(window_str).map_err(|e| OpsError(e.to_string()))?;
    get_x11_window_info(window).map_err(|e| OpsError(e.to_string()))
}

/// Parse window ID string to native handle
pub fn parse_window_string(window_str: &str) -> Result<Window> {
    parse_window_id(window_str).map_err(|e| OpsError(e.to_string()))
}

// ============================================================================
// Screenshot Operations
// ============================================================================

/// Take a screenshot of a window
pub fn take_screenshot(window_str: &str, method: Option<&str>) -> Result<Screenshot> {
    let window = parse_window_id(window_str).map_err(|e| OpsError(e.to_string()))?;
    let method = method
        .and_then(|m| ScreenshotMethod::from_str(m))
        .unwrap_or_default();

    let screenshot = capture_screenshot(window, method).map_err(|e| OpsError(e.to_string()))?;

    Ok(Screenshot {
        base64_image: screenshot.base64_image,
        width: screenshot.width,
        height: screenshot.height,
        format: screenshot.format,
    })
}

// ============================================================================
// Accessibility Operations (AT-SPI2)
// ============================================================================

/// Dump the accessibility element tree
pub fn dump_tree(window_str: &str, max_depth: u32) -> Result<UiaElement> {
    let window = parse_window_id(window_str).map_err(|e| OpsError(e.to_string()))?;

    let conn = AtSpiConnection::new().map_err(|e| OpsError(format!("AT-SPI init failed: {}", e)))?;

    let root = conn
        .get_element_for_window(window)
        .map_err(|e| OpsError(format!("Failed to get window element: {}", e)))?;

    let options = TreeDumpOptions {
        max_depth,
        prune_offscreen: true,
        prune_empty: true,
        max_list_items: 20,
    };

    at_dump_tree(&conn, &root, &options).map_err(|e| OpsError(e.to_string()))
}

/// Find elements by selector (simplified version for AT-SPI)
pub fn find_elements(
    window_str: &str,
    selector_str: &str,
    find_all: bool,
) -> Result<Vec<UiaElement>> {
    let window = parse_window_id(window_str).map_err(|e| OpsError(e.to_string()))?;

    let conn = AtSpiConnection::new().map_err(|e| OpsError(format!("AT-SPI init failed: {}", e)))?;

    let root = conn
        .get_element_for_window(window)
        .map_err(|e| OpsError(format!("Failed to get window element: {}", e)))?;

    // Parse the simple selector
    let (role_filter, name_filter) = parse_simple_selector(selector_str);

    // Search the tree
    let mut results = Vec::new();
    find_elements_recursive(&conn, &root, &role_filter, &name_filter, find_all, &mut results, 0, 20)?;

    Ok(results)
}

/// Parse a simple selector like "Button", "#id", "[name=value]"
fn parse_simple_selector(selector: &str) -> (Option<String>, Option<String>) {
    let selector = selector.trim();

    // Handle [name=value] or [name*=value]
    if selector.starts_with('[') && selector.ends_with(']') {
        let inner = &selector[1..selector.len() - 1];
        if let Some(eq_pos) = inner.find('=') {
            let attr = &inner[..eq_pos].trim_end_matches('*');
            let value = inner[eq_pos + 1..].trim_matches('"').trim_matches('\'');
            if attr.to_lowercase() == "name" {
                return (None, Some(value.to_string()));
            }
        }
        return (None, None);
    }

    // Handle #automationId (treat as name search on Linux)
    if selector.starts_with('#') {
        return (None, Some(selector[1..].to_string()));
    }

    // Handle .className (not well supported on Linux, treat as role)
    if selector.starts_with('.') {
        return (Some(selector[1..].to_string()), None);
    }

    // Handle ControlType (e.g., "Button", "Edit")
    if selector.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        return (Some(selector.to_string()), None);
    }

    // Default: search by name
    (None, Some(selector.to_string()))
}

/// Recursively find elements matching criteria
fn find_elements_recursive(
    conn: &AtSpiConnection,
    elem: &AccessibleElement,
    role_filter: &Option<String>,
    name_filter: &Option<String>,
    find_all: bool,
    results: &mut Vec<UiaElement>,
    depth: u32,
    max_depth: u32,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    // Check if this element matches
    let matches = check_element_match(conn, elem, role_filter, name_filter)?;

    if matches {
        let uia_elem = element_to_uia(conn, elem, depth)?;
        results.push(uia_elem);

        if !find_all {
            return Ok(());
        }
    }

    // Recurse into children
    if let Ok(children) = conn.get_children(elem) {
        for child in children {
            if !find_all && !results.is_empty() {
                break;
            }
            let _ = find_elements_recursive(
                conn,
                &child,
                role_filter,
                name_filter,
                find_all,
                results,
                depth + 1,
                max_depth,
            );
        }
    }

    Ok(())
}

/// Check if an element matches the filter criteria
fn check_element_match(
    conn: &AtSpiConnection,
    elem: &AccessibleElement,
    role_filter: &Option<String>,
    name_filter: &Option<String>,
) -> Result<bool> {
    // Check role filter
    if let Some(ref role) = role_filter {
        let actual_role = conn.get_role(elem).unwrap_or(roles::UNKNOWN);
        let role_str = role_to_string(actual_role);
        if !role_str.eq_ignore_ascii_case(role) {
            return Ok(false);
        }
    }

    // Check name filter
    if let Some(ref name) = name_filter {
        let actual_name = conn.get_name(elem).unwrap_or_default();
        // Support wildcard matching with * at start/end
        let matches = if name.starts_with('*') && name.ends_with('*') {
            actual_name.to_lowercase().contains(&name[1..name.len() - 1].to_lowercase())
        } else if name.starts_with('*') {
            actual_name.to_lowercase().ends_with(&name[1..].to_lowercase())
        } else if name.ends_with('*') {
            actual_name.to_lowercase().starts_with(&name[..name.len() - 1].to_lowercase())
        } else {
            actual_name.eq_ignore_ascii_case(name)
        };

        if !matches {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Convert AccessibleElement to UiaElement
fn element_to_uia(conn: &AtSpiConnection, elem: &AccessibleElement, depth: u32) -> Result<UiaElement> {
    let name = conn.get_name(elem).unwrap_or_default();
    let role = conn.get_role(elem).unwrap_or(roles::UNKNOWN);
    let role_name = conn.get_role_name(elem).unwrap_or_default();
    let states = conn.get_state_set(elem).unwrap_or_default();
    let extents = conn.get_extents(elem).unwrap_or((0, 0, 0, 0));
    let interfaces = conn.get_interfaces(elem).unwrap_or_default();

    // Determine supported patterns from interfaces
    let mut patterns = Vec::new();
    if interfaces.iter().any(|i| i.contains("Action")) {
        patterns.push("Invoke".to_string());
    }
    if interfaces.iter().any(|i| i.contains("Value")) {
        patterns.push("Value".to_string());
    }
    if interfaces.iter().any(|i| i.contains("Text")) {
        patterns.push("Text".to_string());
    }
    if interfaces.iter().any(|i| i.contains("EditableText")) {
        patterns.push("EditableText".to_string());
    }
    if interfaces.iter().any(|i| i.contains("Selection")) {
        patterns.push("Selection".to_string());
    }

    // Get value if available
    let value = if interfaces.iter().any(|i| i.contains("Value")) {
        conn.get_current_value(elem).ok().map(|v| v.to_string())
    } else if interfaces.iter().any(|i| i.contains("Text")) {
        let count = conn.get_character_count(elem).unwrap_or(0);
        if count > 0 && count < 1000 {
            conn.get_text(elem, 0, count).ok()
        } else {
            None
        }
    } else {
        None
    };

    let is_enabled = has_state(&states, states::ENABLED) || has_state(&states, states::SENSITIVE);
    let is_showing = has_state(&states, states::SHOWING);
    let is_visible = has_state(&states, states::VISIBLE);
    let is_offscreen = !is_showing || !is_visible;

    Ok(UiaElement {
        id: elem.to_id_string(),
        control_type: role_to_string(role),
        localized_type: role_name,
        name,
        automation_id: String::new(),
        class_name: String::new(),
        value,
        bounds: [extents.0, extents.1, extents.2, extents.3],
        is_enabled,
        is_offscreen,
        patterns,
        depth,
        children: Vec::new(),
    })
}

/// Check if an element exists in a window
pub fn element_exists(window_str: &str, selector_str: &str) -> Result<bool> {
    match find_elements(window_str, selector_str, false) {
        Ok(elements) => Ok(!elements.is_empty()),
        Err(_) => Ok(false),
    }
}

/// Invoke a pattern on an element
pub fn invoke_pattern(
    window_str: &str,
    selector_str: &str,
    pattern_str: &str,
    value: Option<&str>,
) -> Result<PatternResult> {
    let window = parse_window_id(window_str).map_err(|e| OpsError(e.to_string()))?;

    let conn = AtSpiConnection::new().map_err(|e| OpsError(format!("AT-SPI init failed: {}", e)))?;

    let root = conn
        .get_element_for_window(window)
        .map_err(|e| OpsError(format!("Failed to get window element: {}", e)))?;

    let pattern_op = PatternOp::from_str(pattern_str)
        .ok_or_else(|| OpsError(format!("Unknown pattern: {}", pattern_str)))?;

    // Find element
    let (role_filter, name_filter) = parse_simple_selector(selector_str);
    let mut elements = Vec::new();
    find_elements_recursive(&conn, &root, &role_filter, &name_filter, false, &mut elements, 0, 20)?;

    if elements.is_empty() {
        return Ok(PatternResult::err(format!(
            "No element found matching: {}",
            selector_str
        )));
    }

    // Re-find the AccessibleElement for the matched element
    // (We need the AccessibleElement, not UiaElement, for pattern execution)
    let target = find_accessible_element(&conn, &root, &role_filter, &name_filter, 0, 20)?
        .ok_or_else(|| OpsError("Element not found for pattern execution".to_string()))?;

    Ok(execute_pattern(&conn, &target, pattern_op, value))
}

/// Find an AccessibleElement matching criteria
fn find_accessible_element(
    conn: &AtSpiConnection,
    elem: &AccessibleElement,
    role_filter: &Option<String>,
    name_filter: &Option<String>,
    depth: u32,
    max_depth: u32,
) -> Result<Option<AccessibleElement>> {
    if depth > max_depth {
        return Ok(None);
    }

    if check_element_match(conn, elem, role_filter, name_filter)? {
        return Ok(Some(elem.clone()));
    }

    if let Ok(children) = conn.get_children(elem) {
        for child in children {
            if let Ok(Some(found)) =
                find_accessible_element(conn, &child, role_filter, name_filter, depth + 1, max_depth)
            {
                return Ok(Some(found));
            }
        }
    }

    Ok(None)
}

// ============================================================================
// Summary and Query Operations (LLM-optimized)
// ============================================================================

/// Get a compact UI summary
pub fn get_summary(
    window_str: &str,
    format: &str,
    include_bounds: bool,
    _include_paths: bool,
    _focus_region: Option<[i32; 4]>,
    max_depth: u32,
    _roles: Option<Vec<String>>,
) -> Result<String> {
    let tree = dump_tree(window_str, max_depth)?;

    // Format output
    match format {
        "text" => Ok(format_text_summary(&tree)),
        _ => serde_json::to_string_pretty(&tree)
            .map_err(|e| OpsError(format!("JSON serialization failed: {}", e))),
    }
}

/// Format tree as text summary
fn format_text_summary(elem: &UiaElement) -> String {
    let mut output = String::new();
    format_element_text(&mut output, elem, 0);
    output
}

fn format_element_text(output: &mut String, elem: &UiaElement, indent: usize) {
    let prefix = "  ".repeat(indent);

    // Build element description
    let mut desc = format!("{}{}", prefix, elem.control_type);
    if !elem.name.is_empty() {
        desc.push_str(&format!(" \"{}\"", elem.name));
    }
    if !elem.patterns.is_empty() {
        desc.push_str(&format!(" [{}]", elem.patterns.join(", ")));
    }
    if !elem.is_enabled {
        desc.push_str(" (disabled)");
    }

    output.push_str(&desc);
    output.push('\n');

    // Recurse into children
    for child in &elem.children {
        format_element_text(output, child, indent + 1);
    }
}

/// Query elements with enhanced LLM syntax
pub fn query_elements(window_str: &str, query_str: &str, find_all: bool) -> Result<QueryResult> {
    let elements = find_elements(window_str, query_str, find_all)?;

    // Convert to ElementRef
    let matches: Vec<ElementRef> = elements
        .iter()
        .enumerate()
        .map(|(i, elem)| {
            let role_prefix = match elem.control_type.as_str() {
                "Button" => "b",
                "Edit" | "Text" => "t",
                "CheckBox" => "c",
                "RadioButton" => "r",
                "List" | "ListItem" => "l",
                "Menu" | "MenuItem" => "m",
                "Tab" | "TabItem" => "tab",
                _ => "e",
            };

            ElementRef {
                id: format!("{}{}", role_prefix, i + 1),
                role: elem.control_type.clone(),
                label: if !elem.name.is_empty() {
                    elem.name.clone()
                } else {
                    elem.control_type.clone()
                },
                action: if elem.patterns.contains(&"Invoke".to_string()) {
                    Some("click".to_string())
                } else if elem.patterns.contains(&"Value".to_string())
                    || elem.patterns.contains(&"EditableText".to_string())
                {
                    Some("type".to_string())
                } else {
                    None
                },
                selector: format!("[name=\"{}\"]", elem.name),
            }
        })
        .collect();

    Ok(QueryResult {
        count: matches.len(),
        matches,
        suggestions: Vec::new(),
    })
}

// ============================================================================
// Input Operations
// ============================================================================

/// Click at coordinates or on an element
pub fn click(
    window_str: &str,
    click_type: &str,
    coords: Option<(i32, i32)>,
    selector: Option<&str>,
) -> Result<()> {
    let window = parse_window_id(window_str).map_err(|e| OpsError(e.to_string()))?;

    // Determine if we're using screen coordinates (selector) or window-relative coords
    let (x, y, is_screen_coords) = if let Some((cx, cy)) = coords {
        // User-provided coords are window-relative
        (cx, cy, false)
    } else if let Some(selector_str) = selector {
        // Find element center - AT-SPI bounds are in screen coordinates
        let elements = find_elements(window_str, selector_str, false)?;
        if elements.is_empty() {
            return Err(OpsError(format!(
                "No element found matching: {}",
                selector_str
            )));
        }
        let bounds = elements[0].bounds;
        // bounds[0]=x, bounds[1]=y, bounds[2]=width, bounds[3]=height
        // These are already in screen coordinates from AT-SPI GetExtents
        (bounds[0] + bounds[2] / 2, bounds[1] + bounds[3] / 2, true)
    } else {
        return Err(OpsError(
            "Either coords or selector must be specified".to_string(),
        ));
    };

    if is_screen_coords {
        // Use screen coordinate functions (no window offset conversion)
        match click_type.to_lowercase().as_str() {
            "right" => right_click_at_screen_coords(x, y).map_err(|e| OpsError(e.to_string())),
            "double" => double_click_at_screen_coords(x, y).map_err(|e| OpsError(e.to_string())),
            _ => click_at_screen_coords(x, y).map_err(|e| OpsError(e.to_string())),
        }
    } else {
        // Use window-relative functions (converts to screen coords internally)
        match click_type.to_lowercase().as_str() {
            "right" => right_click_at_coords(window, x, y).map_err(|e| OpsError(e.to_string())),
            "double" => double_click_at_coords(window, x, y).map_err(|e| OpsError(e.to_string())),
            _ => click_at_coords(window, x, y).map_err(|e| OpsError(e.to_string())),
        }
    }
}

/// Type text (optionally after clicking on a selector)
pub fn type_text(window_str: &str, text: &str, selector: Option<&str>) -> Result<()> {
    if let Some(selector_str) = selector {
        // Click to focus first
        click(window_str, "left", None, Some(selector_str))?;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    input_type_text(text).map_err(|e| OpsError(e.to_string()))
}

/// Send key combination
pub fn send_keys(keys: &str) -> Result<()> {
    input_send_keys(keys).map_err(|e| OpsError(e.to_string()))
}

/// Scroll up or down
pub fn scroll(direction: &str, amount: i32) -> Result<()> {
    let scroll_amount = match direction.to_lowercase().as_str() {
        "up" => amount,
        "down" => -amount,
        _ => return Err(OpsError(format!("Invalid direction: {}", direction))),
    };

    input_scroll(scroll_amount).map_err(|e| OpsError(e.to_string()))
}
