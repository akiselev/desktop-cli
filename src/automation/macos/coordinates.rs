//! macOS coordinate translation utilities

use crate::automation::macos::window::{get_window_info, WindowId};
use crate::error::{DesktopCliError, Result};
use core_graphics::display::{CGDisplay, CGMainDisplayID};

/// Convert window-relative coordinates to screen coordinates
pub fn window_to_screen_coords(window: WindowId, window_x: i32, window_y: i32) -> Result<(i32, i32)> {
    // Get window position from window info
    let info = get_window_info(window)?;

    // Window coordinates are relative to window, add window position
    let screen_x = info.rect.x + window_x;
    let screen_y = info.rect.y + window_y;

    Ok((screen_x, screen_y))
}

/// Get screen dimensions
pub fn get_screen_size() -> Result<(u32, u32)> {
    let main_display = unsafe { CGMainDisplayID() };
    let display = CGDisplay::new(main_display);

    let width = display.pixels_wide();
    let height = display.pixels_high();

    if width == 0 || height == 0 {
        return Err(DesktopCliError::CoordinateError(
            "Failed to get screen dimensions".to_string(),
        ));
    }

    Ok((width as u32, height as u32))
}

/// Calculate center point of a bounding box
pub fn calculate_center(x: u32, y: u32, width: u32, height: u32) -> (i32, i32) {
    let center_x = x as i32 + (width as i32 / 2);
    let center_y = y as i32 + (height as i32 / 2);
    (center_x, center_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_center() {
        assert_eq!(calculate_center(100, 100, 80, 40), (140, 120));
        assert_eq!(calculate_center(0, 0, 100, 100), (50, 50));
    }
}
