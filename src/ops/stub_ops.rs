//! Stub implementations for non-Windows platforms

use crate::automation::types::WindowInfo;
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

fn not_supported<T>() -> Result<T> {
    Err(OpsError("Platform not supported".to_string()))
}

pub fn list_windows(_: Option<&str>, _: Option<&str>) -> Result<Vec<WindowInfo>> {
    not_supported()
}

pub fn get_window_by_hwnd(_: &str) -> Result<WindowInfo> {
    not_supported()
}

pub fn take_screenshot(_: &str, _: Option<&str>) -> Result<Screenshot> {
    not_supported()
}

pub fn dump_tree(_: &str, _: u32) -> Result<UiaElement> {
    not_supported()
}

pub fn find_elements(_: &str, _: &str, _: bool) -> Result<Vec<UiaElement>> {
    not_supported()
}

pub fn element_exists(_: &str, _: &str) -> Result<bool> {
    not_supported()
}

pub fn invoke_pattern(_: &str, _: &str, _: &str, _: Option<&str>) -> Result<PatternResult> {
    not_supported()
}

pub fn get_summary(
    _: &str,
    _: &str,
    _: bool,
    _: bool,
    _: Option<[i32; 4]>,
    _: u32,
    _: Option<Vec<String>>,
) -> Result<String> {
    not_supported()
}

pub fn query_elements(_: &str, _: &str, _: bool) -> Result<QueryResult> {
    not_supported()
}

pub fn click(_: &str, _: &str, _: Option<(i32, i32)>, _: Option<&str>) -> Result<()> {
    not_supported()
}

pub fn type_text(_: &str, _: &str, _: Option<&str>) -> Result<()> {
    not_supported()
}

pub fn send_keys(_: &str, _: &str) -> Result<()> {
    not_supported()
}

pub fn scroll(_: &str, _: &str, _: i32) -> Result<()> {
    not_supported()
}
