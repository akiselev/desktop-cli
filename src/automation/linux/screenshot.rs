//! Screenshot capture (deferred - xcap removed due to dependency issues)

use crate::error::{DesktopCliError, Result};
use crate::rpc::types::Screenshot;

pub fn capture_window(_window_id: &str) -> Result<Screenshot> {
    Err(DesktopCliError::Platform(
        "Screenshot support not yet implemented for Linux".to_string(),
    ))
}
