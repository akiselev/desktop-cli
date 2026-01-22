//! Linux screenshot capture via X11

use crate::automation::linux::coordinates::get_window_geometry;
use crate::error::{DesktopCliError, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use x11::xlib::{
    Display, Window, XCloseDisplay, XDestroyImage, XGetImage, XGetWindowAttributes, XOpenDisplay,
    XWindowAttributes, ZPixmap,
};

/// Screenshot method to use
#[derive(Debug, Clone, Copy)]
pub enum ScreenshotMethod {
    /// Standard X11 XGetImage method
    XGetImage,
    /// Composite extension method (for composited windows)
    Composite,
}

impl Default for ScreenshotMethod {
    fn default() -> Self {
        Self::XGetImage
    }
}

impl ScreenshotMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "xgetimage" | "x11" => Some(Self::XGetImage),
            "composite" => Some(Self::Composite),
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
pub fn capture_screenshot(window: Window, method: ScreenshotMethod) -> Result<Screenshot> {
    // Try primary method first
    let result = match method {
        ScreenshotMethod::XGetImage => capture_xgetimage(window),
        ScreenshotMethod::Composite => capture_composite(window),
    };

    // If primary method fails, try fallback
    result.or_else(|e| {
        tracing::warn!(
            "Primary screenshot method {:?} failed: {}, trying fallback",
            method,
            e
        );
        match method {
            ScreenshotMethod::XGetImage => capture_composite(window),
            ScreenshotMethod::Composite => capture_xgetimage(window),
        }
    })
}

/// Capture screenshot using XGetImage
fn capture_xgetimage(window: Window) -> Result<Screenshot> {
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(DesktopCliError::ScreenshotError(
                "Failed to open X11 display".to_string(),
            ));
        }

        // Get window attributes
        let mut attrs: XWindowAttributes = std::mem::zeroed();
        if XGetWindowAttributes(display, window, &mut attrs) == 0 {
            XCloseDisplay(display);
            return Err(DesktopCliError::ScreenshotError(
                "Failed to get window attributes".to_string(),
            ));
        }

        let width = attrs.width as u32;
        let height = attrs.height as u32;

        if width == 0 || height == 0 {
            XCloseDisplay(display);
            return Err(DesktopCliError::ScreenshotError(
                "Window has zero dimensions".to_string(),
            ));
        }

        // Get the image
        let image = XGetImage(
            display,
            window,
            0,
            0,
            width,
            height,
            0xFFFFFFFF, // AllPlanes
            ZPixmap,
        );

        if image.is_null() {
            XCloseDisplay(display);
            return Err(DesktopCliError::ScreenshotError(
                "Failed to capture window image".to_string(),
            ));
        }

        // Convert image data to RGBA
        let img = &*image;
        let bytes_per_pixel = (img.bits_per_pixel / 8) as usize;
        let row_stride = img.bytes_per_line as usize;

        // Validate bytes_per_pixel is at least 3 (RGB)
        if bytes_per_pixel < 3 {
            XDestroyImage(image);
            XCloseDisplay(display);
            return Err(DesktopCliError::ScreenshotError(format!(
                "Unsupported pixel format: {} bits per pixel",
                img.bits_per_pixel
            )));
        }

        // Calculate total data size and validate
        let total_data_size = match row_stride.checked_mul(height as usize) {
            Some(size) => size,
            None => {
                XDestroyImage(image);
                XCloseDisplay(display);
                return Err(DesktopCliError::ScreenshotError(
                    "Image size overflow".to_string(),
                ));
            }
        };

        // Ensure we don't overflow when calculating required capacity
        let rgba_size = match (width as usize)
            .checked_mul(height as usize)
            .and_then(|v| v.checked_mul(4))
        {
            Some(size) => size,
            None => {
                XDestroyImage(image);
                XCloseDisplay(display);
                return Err(DesktopCliError::ScreenshotError(
                    "RGBA buffer size overflow".to_string(),
                ));
            }
        };

        let mut rgba_data = Vec::with_capacity(rgba_size);

        for y in 0..height {
            for x in 0..width {
                let offset = y as usize * row_stride + x as usize * bytes_per_pixel;

                // Bounds check before accessing pixel data
                let max_offset = if bytes_per_pixel == 4 {
                    offset + 3
                } else {
                    offset + 2
                };
                if max_offset >= total_data_size {
                    XDestroyImage(image);
                    XCloseDisplay(display);
                    return Err(DesktopCliError::ScreenshotError(
                        "Image data buffer overflow detected".to_string(),
                    ));
                }

                // X11 typically uses BGRA format
                let b = *img.data.add(offset) as u8;
                let g = *img.data.add(offset + 1) as u8;
                let r = *img.data.add(offset + 2) as u8;
                let a = if bytes_per_pixel == 4 {
                    *img.data.add(offset + 3) as u8
                } else {
                    255
                };

                rgba_data.push(r);
                rgba_data.push(g);
                rgba_data.push(b);
                rgba_data.push(a);
            }
        }

        XDestroyImage(image);
        XCloseDisplay(display);

        // Encode to PNG
        encode_to_png(width, height, &rgba_data)
    }
}

/// Capture screenshot using Composite extension (placeholder)
fn capture_composite(window: Window) -> Result<Screenshot> {
    // For now, fall back to XGetImage
    // Full composite support would require linking to Xcomposite extension
    capture_xgetimage(window)
}

/// Encode RGBA pixel data to PNG and return as Screenshot
fn encode_to_png(width: u32, height: u32, rgba_data: &[u8]) -> Result<Screenshot> {
    let mut png_bytes = Vec::new();

    {
        let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder
            .write_header()
            .map_err(|e| DesktopCliError::ScreenshotError(format!("PNG header failed: {}", e)))?;

        writer
            .write_image_data(rgba_data)
            .map_err(|e| DesktopCliError::ScreenshotError(format!("PNG encoding failed: {}", e)))?;
    }

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
            ScreenshotMethod::from_str("xgetimage"),
            Some(ScreenshotMethod::XGetImage)
        ));
        assert!(matches!(
            ScreenshotMethod::from_str("composite"),
            Some(ScreenshotMethod::Composite)
        ));
        assert!(ScreenshotMethod::from_str("invalid").is_none());
    }

    #[test]
    fn test_default_screenshot_method() {
        assert!(matches!(
            ScreenshotMethod::default(),
            ScreenshotMethod::XGetImage
        ));
    }
}
