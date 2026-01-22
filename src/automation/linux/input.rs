//! Linux input simulation via XTest extension
//!
//! Uses the X11 Test Extension (XTest) to simulate:
//! - Mouse movement and clicks
//! - Keyboard input
//! - Scroll events

use crate::automation::linux::coordinates::window_to_screen_coords;
use crate::error::{DesktopCliError, Result};
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_int, c_uint};
use std::thread::sleep;
use std::time::Duration;
use x11::xlib::{
    self, Display, Window, XCloseDisplay, XDefaultRootWindow, XFlush, XKeysymToKeycode,
    XOpenDisplay, XSync,
};
use x11::xtest::{XTestFakeButtonEvent, XTestFakeKeyEvent, XTestFakeMotionEvent};

/// X11 button constants
const BUTTON_LEFT: c_uint = 1;
const BUTTON_MIDDLE: c_uint = 2;
const BUTTON_RIGHT: c_uint = 3;
const BUTTON_SCROLL_UP: c_uint = 4;
const BUTTON_SCROLL_DOWN: c_uint = 5;

/// X11 connection for input simulation
struct InputConnection {
    display: *mut Display,
}

impl InputConnection {
    fn new() -> Result<Self> {
        let display = unsafe { XOpenDisplay(std::ptr::null()) };
        if display.is_null() {
            return Err(DesktopCliError::AutomationError(
                "Failed to open X11 display for input simulation".to_string(),
            ));
        }
        Ok(Self { display })
    }

    fn flush(&self) {
        unsafe {
            XFlush(self.display);
        }
    }

    fn sync(&self) {
        unsafe {
            XSync(self.display, 0);
        }
    }
}

impl Drop for InputConnection {
    fn drop(&mut self) {
        if !self.display.is_null() {
            unsafe { XCloseDisplay(self.display) };
        }
    }
}

/// Move the mouse to the specified screen coordinates
fn move_mouse(conn: &InputConnection, x: i32, y: i32) -> Result<()> {
    unsafe {
        let screen = xlib::XDefaultScreen(conn.display);
        let result = XTestFakeMotionEvent(conn.display, screen, x, y, 0);
        if result == 0 {
            return Err(DesktopCliError::AutomationError(
                "Failed to move mouse".to_string(),
            ));
        }
        conn.flush();
    }
    Ok(())
}

/// Press or release a mouse button
fn button_event(conn: &InputConnection, button: c_uint, is_press: bool) -> Result<()> {
    unsafe {
        let result = XTestFakeButtonEvent(conn.display, button, if is_press { 1 } else { 0 }, 0);
        if result == 0 {
            return Err(DesktopCliError::AutomationError(
                "Failed to simulate button event".to_string(),
            ));
        }
        conn.flush();
    }
    Ok(())
}

/// Click at the specified window-relative coordinates
pub fn click_at_coords(window: Window, window_x: i32, window_y: i32) -> Result<()> {
    let conn = InputConnection::new()?;

    // Convert window-relative to screen coordinates
    let (screen_x, screen_y) = window_to_screen_coords(window, window_x, window_y)?;

    // Move mouse to position
    move_mouse(&conn, screen_x, screen_y)?;

    // Small delay for position to register
    sleep(Duration::from_millis(10));

    // Click
    button_event(&conn, BUTTON_LEFT, true)?;
    sleep(Duration::from_millis(10));
    button_event(&conn, BUTTON_LEFT, false)?;

    conn.sync();
    tracing::debug!(
        "Clicked at window coords ({}, {}), screen coords ({}, {})",
        window_x,
        window_y,
        screen_x,
        screen_y
    );

    Ok(())
}

/// Double-click at the specified window-relative coordinates
pub fn double_click_at_coords(window: Window, window_x: i32, window_y: i32) -> Result<()> {
    let conn = InputConnection::new()?;

    // Convert window-relative to screen coordinates
    let (screen_x, screen_y) = window_to_screen_coords(window, window_x, window_y)?;

    // Move mouse to position
    move_mouse(&conn, screen_x, screen_y)?;

    // First click
    sleep(Duration::from_millis(10));
    button_event(&conn, BUTTON_LEFT, true)?;
    sleep(Duration::from_millis(10));
    button_event(&conn, BUTTON_LEFT, false)?;

    // Small delay between clicks
    sleep(Duration::from_millis(50));

    // Second click
    button_event(&conn, BUTTON_LEFT, true)?;
    sleep(Duration::from_millis(10));
    button_event(&conn, BUTTON_LEFT, false)?;

    conn.sync();
    tracing::debug!("Double-clicked at window coords ({}, {})", window_x, window_y);

    Ok(())
}

/// Right-click at the specified window-relative coordinates
pub fn right_click_at_coords(window: Window, window_x: i32, window_y: i32) -> Result<()> {
    let conn = InputConnection::new()?;

    // Convert window-relative to screen coordinates
    let (screen_x, screen_y) = window_to_screen_coords(window, window_x, window_y)?;

    // Move mouse to position
    move_mouse(&conn, screen_x, screen_y)?;

    // Right click
    sleep(Duration::from_millis(10));
    button_event(&conn, BUTTON_RIGHT, true)?;
    sleep(Duration::from_millis(10));
    button_event(&conn, BUTTON_RIGHT, false)?;

    conn.sync();
    tracing::debug!("Right-clicked at window coords ({}, {})", window_x, window_y);

    Ok(())
}

/// Scroll at the current mouse position
/// amount: positive = scroll up, negative = scroll down (in notch units)
pub fn scroll(amount: i32) -> Result<()> {
    let conn = InputConnection::new()?;

    let (button, clicks) = if amount > 0 {
        (BUTTON_SCROLL_UP, amount)
    } else {
        (BUTTON_SCROLL_DOWN, -amount)
    };

    // Each scroll notch is one button press/release
    for _ in 0..clicks {
        button_event(&conn, button, true)?;
        button_event(&conn, button, false)?;
        sleep(Duration::from_millis(10));
    }

    conn.sync();
    tracing::debug!("Scrolled by {}", amount);

    Ok(())
}

/// Get keycode from keysym
fn keysym_to_keycode(conn: &InputConnection, keysym: u64) -> Option<u8> {
    unsafe {
        let keycode = XKeysymToKeycode(conn.display, keysym);
        if keycode == 0 {
            None
        } else {
            Some(keycode)
        }
    }
}

/// Press or release a key
fn key_event(conn: &InputConnection, keycode: u8, is_press: bool) -> Result<()> {
    unsafe {
        let result =
            XTestFakeKeyEvent(conn.display, keycode as c_uint, if is_press { 1 } else { 0 }, 0);
        if result == 0 {
            return Err(DesktopCliError::AutomationError(
                "Failed to simulate key event".to_string(),
            ));
        }
        conn.flush();
    }
    Ok(())
}

/// Type text at the current cursor position using XTest
pub fn type_text(text: &str) -> Result<()> {
    let conn = InputConnection::new()?;

    for ch in text.chars() {
        // Use Unicode input method via XIM or fallback to keysym
        let keysym = char_to_keysym(ch);

        // Check if we need shift
        let needs_shift = ch.is_ascii_uppercase()
            || "~!@#$%^&*()_+{}|:\"<>?".contains(ch);

        if let Some(keycode) = keysym_to_keycode(&conn, keysym) {
            // Press shift if needed
            if needs_shift {
                if let Some(shift_code) = keysym_to_keycode(&conn, 0xFFE1) {
                    // Shift_L
                    key_event(&conn, shift_code, true)?;
                }
            }

            // Press and release the key
            key_event(&conn, keycode, true)?;
            key_event(&conn, keycode, false)?;

            // Release shift if needed
            if needs_shift {
                if let Some(shift_code) = keysym_to_keycode(&conn, 0xFFE1) {
                    key_event(&conn, shift_code, false)?;
                }
            }
        } else {
            // Fallback: use XSendEvent or print warning
            tracing::warn!("No keycode found for character: {}", ch);
        }

        // Small delay between keystrokes
        sleep(Duration::from_millis(5));
    }

    conn.sync();
    tracing::debug!("Typed text: {}", text);

    Ok(())
}

/// Convert character to X11 keysym
fn char_to_keysym(ch: char) -> u64 {
    // For ASCII characters, keysym is usually the character code
    if ch.is_ascii() {
        let c = ch as u64;
        // Lowercase letters
        if ch.is_ascii_lowercase() {
            return c;
        }
        // Uppercase letters - same keysym, but needs shift
        if ch.is_ascii_uppercase() {
            return ch.to_ascii_lowercase() as u64;
        }
        // Numbers and symbols
        match ch {
            '0'..='9' => c,
            ' ' => 0x20,
            '!' => 0x21,
            '"' => 0x22,
            '#' => 0x23,
            '$' => 0x24,
            '%' => 0x25,
            '&' => 0x26,
            '\'' => 0x27,
            '(' => 0x28,
            ')' => 0x29,
            '*' => 0x2A,
            '+' => 0x2B,
            ',' => 0x2C,
            '-' => 0x2D,
            '.' => 0x2E,
            '/' => 0x2F,
            ':' => 0x3A,
            ';' => 0x3B,
            '<' => 0x3C,
            '=' => 0x3D,
            '>' => 0x3E,
            '?' => 0x3F,
            '@' => 0x40,
            '[' => 0x5B,
            '\\' => 0x5C,
            ']' => 0x5D,
            '^' => 0x5E,
            '_' => 0x5F,
            '`' => 0x60,
            '{' => 0x7B,
            '|' => 0x7C,
            '}' => 0x7D,
            '~' => 0x7E,
            '\n' => 0xFF0D, // Return
            '\t' => 0xFF09, // Tab
            _ => c,
        }
    } else {
        // For Unicode, use XK_Unicode + codepoint
        0x01000000 | (ch as u64)
    }
}

/// Get keysym for named key
fn get_named_keysym(key_name: &str) -> Option<u64> {
    let key_map: HashMap<&str, u64> = [
        // Letters (handled separately)
        // Numbers (handled separately)
        // Function keys
        ("f1", 0xFFBE),
        ("f2", 0xFFBF),
        ("f3", 0xFFC0),
        ("f4", 0xFFC1),
        ("f5", 0xFFC2),
        ("f6", 0xFFC3),
        ("f7", 0xFFC4),
        ("f8", 0xFFC5),
        ("f9", 0xFFC6),
        ("f10", 0xFFC7),
        ("f11", 0xFFC8),
        ("f12", 0xFFC9),
        // Modifiers
        ("ctrl", 0xFFE3),
        ("control", 0xFFE3),
        ("lctrl", 0xFFE3),
        ("rctrl", 0xFFE4),
        ("alt", 0xFFE9),
        ("lalt", 0xFFE9),
        ("ralt", 0xFFEA),
        ("shift", 0xFFE1),
        ("lshift", 0xFFE1),
        ("rshift", 0xFFE2),
        ("super", 0xFFEB),
        ("win", 0xFFEB),
        ("meta", 0xFFE7),
        // Special keys
        ("enter", 0xFF0D),
        ("return", 0xFF0D),
        ("tab", 0xFF09),
        ("escape", 0xFF1B),
        ("esc", 0xFF1B),
        ("space", 0x20),
        ("spacebar", 0x20),
        ("backspace", 0xFF08),
        ("back", 0xFF08),
        ("delete", 0xFFFF),
        ("del", 0xFFFF),
        ("insert", 0xFF63),
        ("ins", 0xFF63),
        ("home", 0xFF50),
        ("end", 0xFF57),
        ("pageup", 0xFF55),
        ("pgup", 0xFF55),
        ("pagedown", 0xFF56),
        ("pgdn", 0xFF56),
        // Arrow keys
        ("up", 0xFF52),
        ("down", 0xFF54),
        ("left", 0xFF51),
        ("right", 0xFF53),
        // Other
        ("printscreen", 0xFF61),
        ("prtsc", 0xFF61),
        ("pause", 0xFF13),
        ("capslock", 0xFFE5),
        ("caps", 0xFFE5),
        ("numlock", 0xFF7F),
        ("scrolllock", 0xFF14),
        ("menu", 0xFF67),
    ]
    .iter()
    .cloned()
    .collect();

    let key_lower = key_name.to_lowercase();

    // Check if it's a single letter
    if key_lower.len() == 1 {
        let ch = key_lower.chars().next().unwrap();
        if ch.is_ascii_alphabetic() {
            return Some(ch as u64);
        }
        if ch.is_ascii_digit() {
            return Some(ch as u64);
        }
    }

    key_map.get(key_lower.as_str()).copied()
}

/// Send a key combination (e.g., "ctrl+c", "alt+f4", "ctrl+shift+s")
pub fn send_keys(keys: &str) -> Result<()> {
    let conn = InputConnection::new()?;

    let parts: Vec<&str> = keys.split('+').map(|s| s.trim()).collect();

    if parts.is_empty() {
        return Err(DesktopCliError::AutomationError(
            "Empty key combination".to_string(),
        ));
    }

    // Parse all keysyms
    let mut keysyms: Vec<u64> = Vec::new();
    for part in &parts {
        if let Some(ks) = get_named_keysym(part) {
            keysyms.push(ks);
        } else {
            return Err(DesktopCliError::AutomationError(format!(
                "Unknown key: '{}'. Use names like: ctrl, alt, shift, enter, tab, esc, f1-f12, a-z, 0-9",
                part
            )));
        }
    }

    // Convert to keycodes
    let mut keycodes: Vec<u8> = Vec::new();
    for ks in &keysyms {
        if let Some(kc) = keysym_to_keycode(&conn, *ks) {
            keycodes.push(kc);
        } else {
            return Err(DesktopCliError::AutomationError(format!(
                "No keycode for keysym: {}",
                ks
            )));
        }
    }

    // Press all keys down (in order)
    for &kc in &keycodes {
        key_event(&conn, kc, true)?;
    }

    // Release all keys (in reverse order)
    for &kc in keycodes.iter().rev() {
        key_event(&conn, kc, false)?;
    }

    conn.sync();
    tracing::debug!("Sent key combination: {}", keys);

    Ok(())
}

/// Press a specific key by keysym
pub fn press_key(keysym: u64) -> Result<()> {
    let conn = InputConnection::new()?;

    if let Some(keycode) = keysym_to_keycode(&conn, keysym) {
        key_event(&conn, keycode, true)?;
        key_event(&conn, keycode, false)?;
        conn.sync();
        tracing::debug!("Pressed key with keysym: {}", keysym);
        Ok(())
    } else {
        Err(DesktopCliError::AutomationError(format!(
            "No keycode for keysym: {}",
            keysym
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_keysym() {
        assert_eq!(char_to_keysym('a'), 0x61);
        assert_eq!(char_to_keysym('A'), 0x61); // Same keysym, shift needed
        assert_eq!(char_to_keysym('1'), 0x31);
        assert_eq!(char_to_keysym(' '), 0x20);
        assert_eq!(char_to_keysym('\n'), 0xFF0D);
    }

    #[test]
    fn test_get_named_keysym() {
        assert_eq!(get_named_keysym("ctrl"), Some(0xFFE3));
        assert_eq!(get_named_keysym("enter"), Some(0xFF0D));
        assert_eq!(get_named_keysym("a"), Some(0x61));
        assert_eq!(get_named_keysym("invalid"), None);
    }
}
