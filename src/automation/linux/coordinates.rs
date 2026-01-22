//! Linux coordinate translation utilities

use crate::error::{DesktopCliError, Result};
use x11::xlib::{
    Display, Window, XCloseDisplay, XGetWindowAttributes, XOpenDisplay, XTranslateCoordinates,
    XWindowAttributes,
};

/// Convert window-relative coordinates to screen coordinates
pub fn window_to_screen_coords(window: Window, window_x: i32, window_y: i32) -> Result<(i32, i32)> {
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(DesktopCliError::CoordinateError(
                "Failed to open X11 display".to_string(),
            ));
        }

        // Get root window for translation
        let root = x11::xlib::XDefaultRootWindow(display);

        // Translate coordinates from window to root (screen) coordinates
        let mut screen_x: i32 = 0;
        let mut screen_y: i32 = 0;
        let mut child: Window = 0;

        let result = XTranslateCoordinates(
            display,
            window,
            root,
            window_x,
            window_y,
            &mut screen_x,
            &mut screen_y,
            &mut child,
        );

        XCloseDisplay(display);

        if result == 0 {
            return Err(DesktopCliError::CoordinateError(
                "Failed to translate coordinates".to_string(),
            ));
        }

        Ok((screen_x, screen_y))
    }
}

/// Get window geometry (position and size)
pub fn get_window_geometry(window: Window) -> Result<(i32, i32, u32, u32)> {
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(DesktopCliError::CoordinateError(
                "Failed to open X11 display".to_string(),
            ));
        }

        let mut attrs: XWindowAttributes = std::mem::zeroed();
        let result = XGetWindowAttributes(display, window, &mut attrs);

        // Translate to get absolute screen position
        let root = x11::xlib::XDefaultRootWindow(display);
        let mut abs_x: i32 = 0;
        let mut abs_y: i32 = 0;
        let mut child: Window = 0;

        XTranslateCoordinates(
            display,
            window,
            root,
            0,
            0,
            &mut abs_x,
            &mut abs_y,
            &mut child,
        );

        XCloseDisplay(display);

        if result == 0 {
            return Err(DesktopCliError::CoordinateError(
                "Failed to get window attributes".to_string(),
            ));
        }

        Ok((abs_x, abs_y, attrs.width as u32, attrs.height as u32))
    }
}

/// Get screen dimensions
pub fn get_screen_size() -> Result<(u32, u32)> {
    unsafe {
        let display = XOpenDisplay(std::ptr::null());
        if display.is_null() {
            return Err(DesktopCliError::CoordinateError(
                "Failed to open X11 display".to_string(),
            ));
        }

        let screen = x11::xlib::XDefaultScreen(display);
        let width = x11::xlib::XDisplayWidth(display, screen);
        let height = x11::xlib::XDisplayHeight(display, screen);

        XCloseDisplay(display);

        if width <= 0 || height <= 0 {
            return Err(DesktopCliError::CoordinateError(
                "Failed to get screen dimensions".to_string(),
            ));
        }

        Ok((width as u32, height as u32))
    }
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
