//! Screenshot capture via xcap

use crate::rpc::types::Screenshot;
use crate::error::{DesktopCliError, Result};
use xcap::Monitor;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;
use super::window::parse_window_id;

pub fn capture_window(window_id: &str) -> Result<Screenshot> {
    let window_id_num = parse_window_id(window_id)?;

    let (conn, _screen_num) = RustConnection::connect(None)
        .map_err(|e| DesktopCliError::Platform(format!("X11 connection failed: {}", e)))?;

    let geometry = conn.get_geometry(window_id_num)
        .map_err(|e| DesktopCliError::Platform(format!("Failed to get window geometry: {}", e)))?
        .reply()
        .map_err(|e| DesktopCliError::Platform(format!("Failed to get geometry reply: {}", e)))?;

    let monitors = Monitor::all()
        .map_err(|e| DesktopCliError::ScreenshotError(format!("Failed to enumerate monitors: {}", e)))?;

    if monitors.is_empty() {
        return Err(DesktopCliError::ScreenshotError("No monitors found".to_string()));
    }

    let monitor = &monitors[0];

    let image = monitor.capture_image()
        .map_err(|e| DesktopCliError::ScreenshotError(format!("Failed to capture screenshot: {}", e)))?;

    let x = geometry.x.max(0) as u32;
    let y = geometry.y.max(0) as u32;
    let width = geometry.width as u32;
    let height = geometry.height as u32;

    let image_data = image.as_raw();
    let image_width = image.width();
    let image_height = image.height();

    if x + width > image_width || y + height > image_height {
        return Err(DesktopCliError::ScreenshotError(format!(
            "Window bounds [{}, {}, {}, {}] exceed monitor dimensions {}x{}",
            x, y, width, height, image_width, image_height
        )));
    }

    let mut cropped_data = Vec::with_capacity((width * height * 4) as usize);
    for row in y..(y + height) {
        let start = ((row * image_width + x) * 4) as usize;
        let end = start + (width * 4) as usize;
        if end <= image_data.len() {
            cropped_data.extend_from_slice(&image_data[start..end]);
        }
    }

    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()
            .map_err(|e| DesktopCliError::ScreenshotError(format!("Failed to write PNG header: {}", e)))?;

        writer.write_image_data(&cropped_data)
            .map_err(|e| DesktopCliError::ScreenshotError(format!("Failed to write PNG data: {}", e)))?;
    }

    let base64_image = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_data);

    Ok(Screenshot {
        base64_image,
        width,
        height,
        format: "png".to_string(),
    })
}
