use crate::error::{DesktopCliError, Result};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, GetWindowRect, SM_CXSCREEN, SM_CYSCREEN,
};

/// Convert window-relative coordinates to screen coordinates
pub fn window_to_screen_coords(hwnd: HWND, window_x: i32, window_y: i32) -> Result<(i32, i32)> {
    unsafe {
        // Get the window rect to find the window's position on screen
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).map_err(|e| {
            DesktopCliError::CoordinateError(format!("GetWindowRect failed: {}", e))
        })?;

        // Window coordinates are relative to the window, screen coordinates add the window position
        let screen_x = rect.left + window_x;
        let screen_y = rect.top + window_y;

        Ok((screen_x, screen_y))
    }
}

/// Normalize screen coordinates to 0-65535 range for SendInput
/// This is required by the Windows SendInput API
pub fn normalize_screen_coords(screen_x: i32, screen_y: i32) -> Result<(i32, i32)> {
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    if screen_width == 0 || screen_height == 0 {
        return Err(DesktopCliError::CoordinateError(
            "Failed to get screen metrics".to_string(),
        ));
    }

    let normalized_x = (screen_x * 65536) / screen_width;
    let normalized_y = (screen_y * 65536) / screen_height;

    Ok((normalized_x, normalized_y))
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

    #[test]
    fn test_normalize_coords_logic() {
        // Test normalization math (using hypothetical screen size)
        let screen_width = 1920;
        let screen_height = 1080;

        let x = 960; // center of screen
        let y = 540;

        let norm_x = (x * 65536) / screen_width;
        let norm_y = (y * 65536) / screen_height;

        // Should be roughly in the middle of 0-65535 range
        assert!(norm_x > 30000 && norm_x < 35000);
        assert!(norm_y > 30000 && norm_y < 35000);
    }
}
