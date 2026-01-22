//! macOS screenshot capture via Core Graphics

use crate::automation::macos::window::WindowId;
use crate::error::{DesktopCliError, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use core_foundation::base::TCFType;
use core_foundation::data::CFData;
use core_graphics::display::{
    kCGWindowImageBoundsIgnoreFraming, kCGWindowImageDefault, kCGWindowListOptionIncludingWindow,
    CGWindowListCreateImage,
};
use core_graphics::geometry::{CGRect, CGPoint, CGSize};

/// Screenshot method to use
#[derive(Debug, Clone, Copy)]
pub enum ScreenshotMethod {
    /// Standard window capture
    Default,
    /// Ignore window frame (content only)
    ContentOnly,
}

impl Default for ScreenshotMethod {
    fn default() -> Self {
        Self::Default
    }
}

impl ScreenshotMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "default" | "normal" => Some(Self::Default),
            "contentonly" | "content" => Some(Self::ContentOnly),
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
pub fn capture_screenshot(window: WindowId, method: ScreenshotMethod) -> Result<Screenshot> {
    let image_option = match method {
        ScreenshotMethod::Default => kCGWindowImageDefault,
        ScreenshotMethod::ContentOnly => kCGWindowImageBoundsIgnoreFraming,
    };

    // Capture the window image
    let image = unsafe {
        CGWindowListCreateImage(
            CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(0.0, 0.0)), // CGRectNull - capture entire window
            kCGWindowListOptionIncludingWindow,
            window,
            image_option,
        )
    };

    if image.is_null() {
        return Err(DesktopCliError::ScreenshotError(
            "Failed to capture window image".to_string(),
        ));
    }

    // Get image dimensions
    let width = unsafe { core_graphics::image::CGImageGetWidth(image) };
    let height = unsafe { core_graphics::image::CGImageGetHeight(image) };

    if width == 0 || height == 0 {
        unsafe { core_foundation::base::CFRelease(image as core_foundation::base::CFTypeRef) };
        return Err(DesktopCliError::ScreenshotError(
            "Window has zero dimensions".to_string(),
        ));
    }

    // Create a bitmap context to get the pixel data
    let color_space = unsafe { core_graphics::color_space::CGColorSpaceCreateDeviceRGB() };
    let bytes_per_row = width * 4;
    let mut pixel_data = vec![0u8; (bytes_per_row * height) as usize];

    let context = unsafe {
        core_graphics::context::CGBitmapContextCreate(
            pixel_data.as_mut_ptr() as *mut _,
            width,
            height,
            8, // bits per component
            bytes_per_row,
            color_space,
            core_graphics::base::kCGImageAlphaPremultipliedLast,
        )
    };

    if context.is_null() {
        unsafe {
            core_foundation::base::CFRelease(image as core_foundation::base::CFTypeRef);
            core_foundation::base::CFRelease(color_space as core_foundation::base::CFTypeRef);
        }
        return Err(DesktopCliError::ScreenshotError(
            "Failed to create bitmap context".to_string(),
        ));
    }

    // Draw the image into the context
    unsafe {
        let rect = CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(width as f64, height as f64));
        core_graphics::context::CGContextDrawImage(context, rect, image);

        // Cleanup
        core_foundation::base::CFRelease(context as core_foundation::base::CFTypeRef);
        core_foundation::base::CFRelease(image as core_foundation::base::CFTypeRef);
        core_foundation::base::CFRelease(color_space as core_foundation::base::CFTypeRef);
    }

    // The pixel data is now in RGBA format (premultiplied alpha)
    // Convert to PNG
    encode_to_png(width as u32, height as u32, &pixel_data)
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
            ScreenshotMethod::from_str("default"),
            Some(ScreenshotMethod::Default)
        ));
        assert!(matches!(
            ScreenshotMethod::from_str("contentonly"),
            Some(ScreenshotMethod::ContentOnly)
        ));
        assert!(ScreenshotMethod::from_str("invalid").is_none());
    }

    #[test]
    fn test_default_screenshot_method() {
        assert!(matches!(
            ScreenshotMethod::default(),
            ScreenshotMethod::Default
        ));
    }
}
