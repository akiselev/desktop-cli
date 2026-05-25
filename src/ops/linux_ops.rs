//! Linux-specific operation implementations

use crate::automation::linux;
use crate::automation::types::WindowInfo;
use crate::ops::traits::DesktopPlatform;
use crate::rpc::types::{PatternResult, QueryResult, Screenshot, UiaElement};

#[derive(Debug)]
pub struct OpsError(pub String);

impl std::fmt::Display for OpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for OpsError {}

pub type Result<T> = std::result::Result<T, OpsError>;

/// Linux platform implementation using AT-SPI2 and X11
pub struct LinuxPlatform;

fn take_screenshot(_hwnd: &str, _method: Option<&str>) -> Result<Screenshot> {
    Err(OpsError(
        "Screenshot functionality deferred to post-release".to_string(),
    ))
}

fn dump_tree(hwnd: &str, max_depth: u32) -> Result<UiaElement> {
    linux::atspi::dump_tree(hwnd, max_depth).map_err(|e| OpsError(e.to_string()))
}

fn find_elements(hwnd: &str, selector: &str, find_all: bool) -> Result<Vec<UiaElement>> {
    linux::atspi::find_elements(hwnd, selector, find_all).map_err(|e| OpsError(e.to_string()))
}

fn element_exists(hwnd: &str, selector: &str) -> Result<bool> {
    linux::atspi::element_exists(hwnd, selector).map_err(|e| OpsError(e.to_string()))
}

fn invoke_pattern(
    hwnd: &str,
    selector: &str,
    pattern: &str,
    action: Option<&str>,
) -> Result<PatternResult> {
    linux::atspi::invoke_pattern(hwnd, selector, pattern, action)
        .map_err(|e| OpsError(e.to_string()))
}

fn get_summary(
    hwnd: &str,
    selector: &str,
    include_invisible: bool,
    include_offscreen: bool,
    bbox: Option<[i32; 4]>,
    max_depth: u32,
    control_types: Option<Vec<String>>,
) -> Result<String> {
    linux::atspi::get_summary(
        hwnd,
        selector,
        include_invisible,
        include_offscreen,
        bbox,
        max_depth,
        control_types,
    )
    .map_err(|e| OpsError(e.to_string()))
}

fn query_elements(hwnd: &str, selector: &str, find_all: bool) -> Result<QueryResult> {
    linux::atspi::query_elements(hwnd, selector, find_all).map_err(|e| OpsError(e.to_string()))
}

fn focus_window(hwnd: &str) -> Result<()> {
    linux::window::focus_window(hwnd).map_err(|e| OpsError(e.to_string()))
}

fn click(
    hwnd: &str,
    selector: &str,
    coords: Option<(i32, i32)>,
    _button: Option<&str>,
) -> Result<()> {
    focus_window(hwnd)?;

    if let Some((x, y)) = coords {
        return linux::input::click_at_coords(x, y).map_err(|e| OpsError(e.to_string()));
    }

    if !selector.is_empty() {
        let elements = find_elements(hwnd, selector, false)?;
        let Some(element) = elements.first() else {
            return Err(OpsError(format!(
                "No element found matching '{}'",
                selector
            )));
        };

        let [x, y, width, height] = element.bounds;
        if width <= 0 || height <= 0 {
            return Err(OpsError(format!(
                "Element '{}' has invalid bounds: {:?}",
                selector, element.bounds
            )));
        }

        let center_x = x + width / 2;
        let center_y = y + height / 2;
        return linux::input::click_at_coords(center_x, center_y)
            .map_err(|e| OpsError(e.to_string()));
    }

    Err(OpsError(
        "Either coordinates or selector required for Linux click".to_string(),
    ))
}

fn type_text(hwnd: &str, text: &str, selector: Option<&str>) -> Result<()> {
    if let Some(selector) = selector.filter(|selector| !selector.is_empty()) {
        click(hwnd, selector, None, None)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
    } else {
        focus_window(hwnd)?;
    }

    linux::input::type_text(text).map_err(|e| OpsError(e.to_string()))
}

fn send_keys(hwnd: &str, keys: &str) -> Result<()> {
    focus_window(hwnd)?;
    linux::input::send_keys(keys).map_err(|e| OpsError(e.to_string()))
}

fn scroll(hwnd: &str, direction: &str, amount: i32) -> Result<()> {
    focus_window(hwnd)?;
    linux::input::scroll(direction, amount).map_err(|e| OpsError(e.to_string()))
}

// ============================================================================
// Public API (delegates to trait implementation)
// ============================================================================

pub fn list_windows_api(
    exe_filter: Option<&str>,
    title_filter: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    LinuxPlatform.list_windows(exe_filter, title_filter)
}

pub fn get_window_by_hwnd_api(hwnd: &str) -> Result<WindowInfo> {
    LinuxPlatform.get_window_by_hwnd(hwnd)
}

pub fn take_screenshot_api(hwnd: &str, method: Option<&str>) -> Result<Screenshot> {
    LinuxPlatform.take_screenshot(hwnd, method)
}

pub fn dump_tree_api(hwnd: &str, max_depth: u32) -> Result<UiaElement> {
    LinuxPlatform.dump_tree(hwnd, max_depth)
}

pub fn find_elements_api(hwnd: &str, selector: &str, find_all: bool) -> Result<Vec<UiaElement>> {
    LinuxPlatform.find_elements(hwnd, selector, find_all)
}

pub fn element_exists_api(hwnd: &str, selector: &str) -> Result<bool> {
    LinuxPlatform.element_exists(hwnd, selector)
}

pub fn invoke_pattern_api(
    hwnd: &str,
    selector: &str,
    pattern: &str,
    action: Option<&str>,
) -> Result<PatternResult> {
    LinuxPlatform.invoke_pattern(hwnd, selector, pattern, action)
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
    LinuxPlatform.get_summary(
        hwnd,
        selector,
        include_invisible,
        include_offscreen,
        bbox,
        max_depth,
        control_types,
    )
}

pub fn query_elements_api(hwnd: &str, selector: &str, find_all: bool) -> Result<QueryResult> {
    LinuxPlatform.query_elements(hwnd, selector, find_all)
}

pub fn click_api(
    hwnd: &str,
    selector: &str,
    coords: Option<(i32, i32)>,
    button: Option<&str>,
) -> Result<()> {
    LinuxPlatform.click(hwnd, selector, coords, button)
}

pub fn type_text_api(hwnd: &str, text: &str, selector: Option<&str>) -> Result<()> {
    LinuxPlatform.type_text(hwnd, text, selector)
}

pub fn send_keys_api(hwnd: &str, keys: &str) -> Result<()> {
    LinuxPlatform.send_keys(hwnd, keys)
}

pub fn scroll_api(hwnd: &str, direction: &str, amount: i32) -> Result<()> {
    LinuxPlatform.scroll(hwnd, direction, amount)
}

// ============================================================================
// Trait Implementation
// ============================================================================

impl DesktopPlatform for LinuxPlatform {
    fn list_windows(
        &self,
        exe_filter: Option<&str>,
        title_filter: Option<&str>,
    ) -> Result<Vec<WindowInfo>> {
        linux::window::list_windows(exe_filter, title_filter).map_err(|e| OpsError(e.to_string()))
    }

    fn get_window_by_hwnd(&self, hwnd: &str) -> Result<WindowInfo> {
        let window_id = u32::from_str_radix(hwnd.trim_start_matches("0x"), 16)
            .map_err(|e| OpsError(format!("Invalid window ID: {}", e)))?;
        linux::window::get_window_info_by_id(window_id).map_err(|e| OpsError(e.to_string()))
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
            selector,
            include_invisible,
            include_offscreen,
            bbox,
            max_depth,
            control_types,
        )
    }

    fn query_elements(&self, hwnd: &str, selector: &str, find_all: bool) -> Result<QueryResult> {
        query_elements(hwnd, selector, find_all)
    }

    fn click(
        &self,
        hwnd: &str,
        selector: &str,
        coords: Option<(i32, i32)>,
        button: Option<&str>,
    ) -> Result<()> {
        click(hwnd, selector, coords, button)
    }

    fn type_text(&self, hwnd: &str, text: &str, selector: Option<&str>) -> Result<()> {
        type_text(hwnd, text, selector)
    }

    fn send_keys(&self, hwnd: &str, keys: &str) -> Result<()> {
        send_keys(hwnd, keys)
    }

    fn scroll(&self, hwnd: &str, direction: &str, amount: i32) -> Result<()> {
        scroll(hwnd, direction, amount)
    }
}
