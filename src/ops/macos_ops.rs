//! macOS-specific operation implementations

use crate::automation::types::WindowInfo;
use crate::ops::traits::DesktopPlatform;
use crate::rpc::types::{PatternResult, QueryResult, Screenshot, UiaElement};

#[cfg(target_os = "macos")]
use crate::automation::macos;

#[derive(Debug)]
pub struct OpsError(pub String);

impl std::fmt::Display for OpsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for OpsError {}

pub type Result<T> = std::result::Result<T, OpsError>;

pub struct MacOSPlatform;

fn not_implemented<T>() -> Result<T> {
    Err(OpsError("Not yet implemented".to_string()))
}

fn platform_not_supported<T>() -> Result<T> {
    Err(OpsError("Platform not supported on this system".to_string()))
}

pub fn list_windows(
    _exe_filter: Option<&str>,
    _title_filter: Option<&str>,
) -> Result<Vec<WindowInfo>> {
    platform_not_supported()
}

pub fn get_window_by_hwnd(_hwnd: &str) -> Result<WindowInfo> {
    platform_not_supported()
}

pub fn take_screenshot(_hwnd: &str, _method: Option<&str>) -> Result<Screenshot> {
    platform_not_supported()
}

pub fn dump_tree(_hwnd: &str, _max_depth: u32) -> Result<UiaElement> {
    platform_not_supported()
}

pub fn find_elements(_hwnd: &str, _selector: &str, _find_all: bool) -> Result<Vec<UiaElement>> {
    platform_not_supported()
}

pub fn element_exists(_hwnd: &str, _selector: &str) -> Result<bool> {
    platform_not_supported()
}

pub fn invoke_pattern(
    _hwnd: &str,
    _selector: &str,
    _pattern: &str,
    _action: Option<&str>,
) -> Result<PatternResult> {
    platform_not_supported()
}

pub fn get_summary(
    _hwnd: &str,
    _selector: &str,
    _include_invisible: bool,
    _include_offscreen: bool,
    _bbox: Option<[i32; 4]>,
    _max_depth: u32,
    _control_types: Option<Vec<String>>,
) -> Result<String> {
    platform_not_supported()
}

pub fn query_elements(_hwnd: &str, _selector: &str, _find_all: bool) -> Result<QueryResult> {
    platform_not_supported()
}

pub fn click(_hwnd: &str, _selector: &str, _coords: Option<(i32, i32)>, _button: Option<&str>) -> Result<()> {
    platform_not_supported()
}

pub fn type_text(_hwnd: &str, _text: &str, _selector: Option<&str>) -> Result<()> {
    platform_not_supported()
}

pub fn send_keys(_keys: &str) -> Result<()> {
    platform_not_supported()
}

pub fn scroll(_direction: &str, _amount: i32) -> Result<()> {
    platform_not_supported()
}

impl DesktopPlatform for MacOSPlatform {
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

    fn click(&self, hwnd: &str, selector: &str, coords: Option<(i32, i32)>, button: Option<&str>) -> Result<()> {
        click(hwnd, selector, coords, button)
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
