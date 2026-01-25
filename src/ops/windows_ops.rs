//! Windows-specific operation implementations
//!
//! Direct implementations of desktop automation operations for Windows.

use crate::automation::types::WindowInfo;
use crate::ops::traits::DesktopPlatform;
use crate::automation::windows::{
    capture_screenshot, get_window_info, list_windows as list_windows_raw, parse_hwnd,
    ScreenshotMethod,
};
use crate::automation::windows::input::{
    click_at_coords, double_click_at_coords, right_click_at_coords,
    scroll as input_scroll, send_keys as input_send_keys, type_text as input_type_text,
};
use crate::automation::windows::uia::{
    self, PatternOp, Selector, SummaryOptions, TreeDumpOptions,
};
use crate::rpc::types::{PatternResult, QueryResult, Screenshot, UiaElement, ElementRef};
use windows::Win32::Foundation::HWND;
use uiautomation::UIAutomation;
use uiautomation::types::Handle;

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
// Platform Implementation
// ============================================================================

/// Windows platform implementation using UI Automation
pub struct WindowsPlatform;

// ============================================================================
// Window Operations
// ============================================================================

fn list_windows(
    exe_filter: Option<&str>,
    title_filter: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    list_windows_raw(exe_filter, title_filter).map_err(|e| OpsError(e.to_string()))
}

fn get_window_by_hwnd(hwnd_str: &str) -> Result<WindowInfo> {
    let hwnd = parse_hwnd(hwnd_str).map_err(|e| OpsError(e.to_string()))?;
    get_window_info(hwnd).map_err(|e| OpsError(e.to_string()))
}

fn parse_hwnd_string(hwnd_str: &str) -> Result<HWND> {
    parse_hwnd(hwnd_str).map_err(|e| OpsError(e.to_string()))
}

// ============================================================================
// Screenshot Operations
// ============================================================================

fn take_screenshot(hwnd_str: &str, method: Option<&str>) -> Result<Screenshot> {
    let hwnd = parse_hwnd(hwnd_str).map_err(|e| OpsError(e.to_string()))?;
    let method = method
        .and_then(|m| ScreenshotMethod::from_str(m))
        .unwrap_or_default();

    let screenshot = capture_screenshot(hwnd, method).map_err(|e| OpsError(e.to_string()))?;

    Ok(Screenshot {
        base64_image: screenshot.base64_image,
        width: screenshot.width,
        height: screenshot.height,
        format: screenshot.format,
    })
}

// ============================================================================
// UIA Operations
// ============================================================================

fn dump_tree(hwnd_str: &str, max_depth: u32) -> Result<UiaElement> {
    let hwnd = parse_hwnd(hwnd_str).map_err(|e| OpsError(e.to_string()))?;

    let automation = UIAutomation::new().map_err(|e| OpsError(format!("UIA init failed: {}", e)))?;

    let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
        .map_err(|e| OpsError(format!("Failed to get window element: {}", e)))?;

    let options = TreeDumpOptions {
        max_depth,
        prune_offscreen: true,
        prune_empty: true,
        max_list_items: 20,
    };

    uia::dump_tree(&automation, &root, &options).map_err(|e| OpsError(e.to_string()))
}

fn find_elements(hwnd_str: &str, selector_str: &str, find_all: bool) -> Result<Vec<UiaElement>> {
    let hwnd = parse_hwnd(hwnd_str).map_err(|e| OpsError(e.to_string()))?;

    let selector =
        Selector::parse(selector_str).map_err(|e| OpsError(format!("Invalid selector: {}", e)))?;

    let automation = UIAutomation::new().map_err(|e| OpsError(format!("UIA init failed: {}", e)))?;

    let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
        .map_err(|e| OpsError(format!("Failed to get window element: {}", e)))?;

    uia::find_elements(&automation, &root, &selector, find_all, 500)
        .map_err(|e| OpsError(e.to_string()))
}

fn element_exists(hwnd_str: &str, selector_str: &str) -> Result<bool> {
    match find_elements(hwnd_str, selector_str, false) {
        Ok(elements) => Ok(!elements.is_empty()),
        Err(_) => Ok(false),
    }
}

fn invoke_pattern(
    hwnd_str: &str,
    selector_str: &str,
    pattern_str: &str,
    value: Option<&str>,
) -> Result<PatternResult> {
    let hwnd = parse_hwnd(hwnd_str).map_err(|e| OpsError(e.to_string()))?;

    let selector =
        Selector::parse(selector_str).map_err(|e| OpsError(format!("Invalid selector: {}", e)))?;

    let pattern_op = PatternOp::from_str(pattern_str)
        .ok_or_else(|| OpsError(format!("Unknown pattern: {}", pattern_str)))?;

    let automation = UIAutomation::new().map_err(|e| OpsError(format!("UIA init failed: {}", e)))?;

    let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
        .map_err(|e| OpsError(format!("Failed to get window element: {}", e)))?;

    // Find element
    let elements = uia::find_elements(&automation, &root, &selector, false, 500)
        .map_err(|e| OpsError(e.to_string()))?;

    if elements.is_empty() {
        return Ok(PatternResult::err(format!(
            "No element found matching: {}",
            selector_str
        )));
    }

    Ok(uia::execute_pattern(&root, pattern_op, value))
}

// ============================================================================
// Summary and Query Operations (LLM-optimized)
// ============================================================================

fn get_summary(
    hwnd_str: &str,
    format: &str,
    include_bounds: bool,
    include_paths: bool,
    focus_region: Option<[i32; 4]>,
    max_depth: u32,
    roles: Option<Vec<String>>,
) -> Result<String> {
    let hwnd = parse_hwnd(hwnd_str).map_err(|e| OpsError(e.to_string()))?;

    let automation = UIAutomation::new().map_err(|e| OpsError(format!("UIA init failed: {}", e)))?;

    let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
        .map_err(|e| OpsError(format!("Failed to get window element: {}", e)))?;

    // Get window title
    let window_title = get_window_info(hwnd)
        .map(|info| info.title)
        .unwrap_or_else(|_| "Unknown Window".to_string());

    // Dump tree
    let options = TreeDumpOptions {
        max_depth,
        prune_offscreen: true,
        prune_empty: true,
        max_list_items: 10,
    };

    let tree =
        uia::dump_tree(&automation, &root, &options).map_err(|e| OpsError(e.to_string()))?;

    // Build summary
    let summary_options = SummaryOptions {
        include_bounds,
        include_paths,
        focus_region,
        max_depth,
        min_size: 5,
        role_filter: roles,
    };

    let summary = uia::generate_summary(&tree, &window_title, &summary_options);

    // Format output
    match format {
        "text" => Ok(uia::format_text_summary(&summary)),
        _ => serde_json::to_string_pretty(&summary)
            .map_err(|e| OpsError(format!("JSON serialization failed: {}", e))),
    }
}

fn query_elements(
    hwnd_str: &str,
    query_str: &str,
    find_all: bool,
) -> Result<QueryResult> {
    let hwnd = parse_hwnd(hwnd_str).map_err(|e| OpsError(e.to_string()))?;

    let query = uia::parse_query(query_str).map_err(|e| OpsError(format!("Invalid query: {}", e)))?;

    let automation = UIAutomation::new().map_err(|e| OpsError(format!("UIA init failed: {}", e)))?;

    let root = uia::element_from_hwnd(&automation, hwnd.0 as isize)
        .map_err(|e| OpsError(format!("Failed to get window element: {}", e)))?;

    // Find elements
    let mut elements =
        uia::find_elements(&automation, &root, &query.selector, find_all || query.index.is_some(), 3000)
            .map_err(|e| OpsError(e.to_string()))?;

    // Apply filters
    if !query.state_filters.is_empty() {
        elements = uia::apply_state_filters(&elements, &query.state_filters);
    }

    if let Some(ref index) = query.index {
        elements = uia::apply_index_filter(elements, index);
    }

    // Convert to ElementRef
    let mut id_gen = uia::RefIdGenerator::new();
    let matches: Vec<ElementRef> = elements
        .iter()
        .map(|elem| {
            let role = uia::infer_role(elem);
            ElementRef {
                id: id_gen.next(role),
                role: role.as_str().to_string(),
                label: if !elem.name.is_empty() {
                    elem.name.clone()
                } else if !elem.automation_id.is_empty() {
                    elem.automation_id.clone()
                } else {
                    role.as_str().to_string()
                },
                action: uia::infer_action(elem),
                selector: uia::generate_selector(elem),
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

fn click(
    hwnd_str: &str,
    click_type: &str,
    coords: Option<(i32, i32)>,
    selector: Option<&str>,
) -> Result<()> {
    let hwnd = parse_hwnd(hwnd_str).map_err(|e| OpsError(e.to_string()))?;

    let (x, y) = if let Some((cx, cy)) = coords {
        (cx, cy)
    } else if let Some(selector_str) = selector {
        // Find element center
        let elements = find_elements(hwnd_str, selector_str, false)?;
        if elements.is_empty() {
            return Err(OpsError(format!(
                "No element found matching: {}",
                selector_str
            )));
        }
        let bounds = elements[0].bounds;
        (bounds[0] + bounds[2] / 2, bounds[1] + bounds[3] / 2)
    } else {
        return Err(OpsError("Either coords or selector must be specified".to_string()));
    };

    match click_type.to_lowercase().as_str() {
        "right" => right_click_at_coords(hwnd, x, y).map_err(|e| OpsError(e.to_string())),
        "double" => double_click_at_coords(hwnd, x, y).map_err(|e| OpsError(e.to_string())),
        _ => click_at_coords(hwnd, x, y).map_err(|e| OpsError(e.to_string())),
    }
}

fn type_text(hwnd_str: &str, text: &str, selector: Option<&str>) -> Result<()> {
    if let Some(selector_str) = selector {
        // Click to focus first
        click(hwnd_str, "left", None, Some(selector_str))?;
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    input_type_text(text).map_err(|e| OpsError(e.to_string()))
}

fn send_keys(keys: &str) -> Result<()> {
    input_send_keys(keys).map_err(|e| OpsError(e.to_string()))
}

fn scroll(direction: &str, amount: i32) -> Result<()> {
    let scroll_amount = match direction.to_lowercase().as_str() {
        "up" => amount * 120,
        "down" => -(amount * 120),
        _ => return Err(OpsError(format!("Invalid direction: {}", direction))),
    };

    input_scroll(scroll_amount).map_err(|e| OpsError(e.to_string()))
}

// ============================================================================
// Public API (delegates to trait implementation)
// ============================================================================

pub fn list_windows_api(
    exe_filter: Option<&str>,
    title_filter: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    WindowsPlatform.list_windows(exe_filter, title_filter)
}

pub fn get_window_by_hwnd_api(hwnd: &str) -> Result<WindowInfo> {
    WindowsPlatform.get_window_by_hwnd(hwnd)
}

pub fn take_screenshot_api(hwnd: &str, method: Option<&str>) -> Result<Screenshot> {
    WindowsPlatform.take_screenshot(hwnd, method)
}

pub fn dump_tree_api(hwnd: &str, max_depth: u32) -> Result<UiaElement> {
    WindowsPlatform.dump_tree(hwnd, max_depth)
}

pub fn find_elements_api(hwnd: &str, selector: &str, find_all: bool) -> Result<Vec<UiaElement>> {
    WindowsPlatform.find_elements(hwnd, selector, find_all)
}

pub fn element_exists_api(hwnd: &str, selector: &str) -> Result<bool> {
    WindowsPlatform.element_exists(hwnd, selector)
}

pub fn invoke_pattern_api(
    hwnd: &str,
    selector: &str,
    pattern: &str,
    action: Option<&str>,
) -> Result<PatternResult> {
    WindowsPlatform.invoke_pattern(hwnd, selector, pattern, action)
}

pub fn get_summary_api(
    hwnd: &str,
    selector: &str,
    include_invisible: bool,
    include_offscreen: bool,
    bbox: Option<[i32; 4]>,
    max_depth: u32,
    control_types: Option<Vec<String>>,
) -> Result<String> {
    WindowsPlatform.get_summary(hwnd, selector, include_invisible, include_offscreen, bbox, max_depth, control_types)
}

pub fn query_elements_api(hwnd: &str, selector: &str, find_all: bool) -> Result<QueryResult> {
    WindowsPlatform.query_elements(hwnd, selector, find_all)
}

pub fn click_api(hwnd: &str, selector: &str, coords: Option<(i32, i32)>, button: Option<&str>) -> Result<()> {
    WindowsPlatform.click(hwnd, selector, coords, button)
}

pub fn type_text_api(hwnd: &str, text: &str, selector: Option<&str>) -> Result<()> {
    WindowsPlatform.type_text(hwnd, text, selector)
}

pub fn send_keys_api(keys: &str) -> Result<()> {
    WindowsPlatform.send_keys(keys)
}

pub fn scroll_api(direction: &str, amount: i32) -> Result<()> {
    WindowsPlatform.scroll(direction, amount)
}

// ============================================================================
// Trait Implementation
// ============================================================================

impl DesktopPlatform for WindowsPlatform {
    fn list_windows(
        &self,
        exe_filter: Option<&str>,
        title_filter: Option<&str>,
    ) -> Result<Vec<WindowInfo>> {
        list_windows(exe_filter, title_filter)
    }

    fn get_window_by_hwnd(&self, hwnd: &str) -> Result<WindowInfo> {
        get_window_by_hwnd(hwnd)
    }

    fn take_screenshot(&self, hwnd: &str, method: Option<&str>) -> Result<Screenshot> {
        take_screenshot(hwnd, method)
    }

    fn dump_tree(&self, hwnd: &str, max_depth: u32) -> Result<UiaElement> {
        dump_tree(hwnd, max_depth)
    }

    fn find_elements(&self, hwnd: &str, selector: &str, find_all: bool) -> Result<Vec<UiaElement>> {
        find_elements(hwnd, selector, find_all)
    }

    fn element_exists(&self, hwnd: &str, selector: &str) -> Result<bool> {
        element_exists(hwnd, selector)
    }

    fn invoke_pattern(
        &self,
        hwnd: &str,
        selector: &str,
        pattern: &str,
        action: Option<&str>,
    ) -> Result<PatternResult> {
        invoke_pattern(hwnd, selector, pattern, action)
    }

    fn get_summary(
        &self,
        hwnd: &str,
        selector: &str,
        include_invisible: bool,
        include_offscreen: bool,
        bbox: Option<[i32; 4]>,
        max_depth: u32,
        control_types: Option<Vec<String>>,
    ) -> Result<String> {
        get_summary(
            hwnd,
            "text",
            !include_invisible,
            !include_offscreen,
            bbox,
            max_depth,
            control_types,
        )
    }

    fn query_elements(&self, hwnd: &str, selector: &str, find_all: bool) -> Result<QueryResult> {
        query_elements(hwnd, selector, find_all)
    }

    fn click(&self, hwnd: &str, selector: &str, coords: Option<(i32, i32)>, button: Option<&str>) -> Result<()> {
        let selector_opt = if selector.is_empty() { None } else { Some(selector) };
        click(hwnd, button.unwrap_or("left"), coords, selector_opt)
    }

    fn type_text(&self, hwnd: &str, text: &str, selector: Option<&str>) -> Result<()> {
        type_text(hwnd, text, selector)
    }

    fn send_keys(&self, keys: &str) -> Result<()> {
        send_keys(keys)
    }

    fn scroll(&self, direction: &str, amount: i32) -> Result<()> {
        scroll(direction, amount)
    }
}
