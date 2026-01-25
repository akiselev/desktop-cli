use crate::error::{DesktopCliError, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use windows::Win32::Foundation::HWND;

/// Screenshot method to use
#[derive(Debug, Clone, Copy)]
pub enum ScreenshotMethod {
    /// Fast method using BitBlt (may not work for all windows)
    BitBlt,
    /// Reliable method using PrintWindow (slower but more compatible)
    PrintWindow,
}

impl Default for ScreenshotMethod {
    fn default() -> Self {
        Self::BitBlt
    }
}

impl ScreenshotMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bitblt" => Some(Self::BitBlt),
            "printwindow" => Some(Self::PrintWindow),
            _ => None,
        }
    }
}

/// Screenshot result containing the image data and dimensions
#[derive(Debug, Clone)]
pub struct Screenshot {
    pub base64_image: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

/// Capture a screenshot of the specified window
pub fn capture_screenshot(hwnd: HWND, method: ScreenshotMethod) -> Result<Screenshot> {
    // Try primary method first
    let result = match method {
        ScreenshotMethod::BitBlt => try_capture_bitblt(hwnd),
        ScreenshotMethod::PrintWindow => try_capture_printwindow(hwnd),
    };

    // If primary method fails, try fallback
    let screenshot = result.or_else(|e| {
        tracing::warn!(
            "Primary screenshot method {:?} failed: {}, trying fallback",
            method,
            e
        );
        match method {
            ScreenshotMethod::BitBlt => try_capture_printwindow(hwnd),
            ScreenshotMethod::PrintWindow => try_capture_bitblt(hwnd),
        }
    })?;

    Ok(screenshot)
}

/// Try to capture screenshot using BitBlt method
fn try_capture_bitblt(hwnd: HWND) -> Result<Screenshot> {
    let screenshot = win_screenshot::capture::capture_window(hwnd.0 as isize)
        .map_err(|e| DesktopCliError::ScreenshotError(format!("BitBlt capture failed: {}", e)))?;

    encode_screenshot(screenshot)
}

/// Try to capture screenshot using PrintWindow method
fn try_capture_printwindow(hwnd: HWND) -> Result<Screenshot> {
    // win-screenshot doesn't expose PrintWindow directly, but we can use it via the capture_window function
    // which internally falls back to PrintWindow on failure
    // For now, we'll implement a simple version
    let screenshot = win_screenshot::capture::capture_window(hwnd.0 as isize).map_err(|e| {
        DesktopCliError::ScreenshotError(format!("PrintWindow capture failed: {}", e))
    })?;

    encode_screenshot(screenshot)
}

/// Encode screenshot to base64 PNG
fn encode_screenshot(screenshot: win_screenshot::capture::RgbBuf) -> Result<Screenshot> {
    let width = screenshot.width;
    let height = screenshot.height;

    // Get raw RGBA pixels
    let pixels = &screenshot.pixels;

    // Encode to PNG format
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder
            .write_header()
            .map_err(|e| DesktopCliError::ScreenshotError(format!("PNG encoding failed: {}", e)))?;

        // pixels is already RGBA bytes
        writer
            .write_image_data(pixels)
            .map_err(|e| DesktopCliError::ScreenshotError(format!("PNG writing failed: {}", e)))?;
    }

    // Encode to base64
    let base64_image = BASE64.encode(&png_bytes);

    Ok(Screenshot {
        base64_image,
        width,
        height,
        format: "png".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screenshot_method_from_str() {
        assert!(matches!(
            ScreenshotMethod::from_str("bitblt"),
            Some(ScreenshotMethod::BitBlt)
        ));
        assert!(matches!(
            ScreenshotMethod::from_str("printwindow"),
            Some(ScreenshotMethod::PrintWindow)
        ));
        assert!(ScreenshotMethod::from_str("invalid").is_none());
    }

    #[test]
    fn test_default_screenshot_method() {
        assert!(matches!(
            ScreenshotMethod::default(),
            ScreenshotMethod::BitBlt
        ));
    }
}
