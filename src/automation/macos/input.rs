//! macOS input simulation via Core Graphics Events
//!
//! Uses CGEvent API to simulate:
//! - Mouse movement and clicks
//! - Keyboard input
//! - Scroll events

use crate::automation::macos::coordinates::window_to_screen_coords;
use crate::automation::macos::window::WindowId;
use crate::error::{DesktopCliError, Result};
use core_foundation::base::TCFType;
use core_foundation::string::CFString;
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGKeyCode, CGMouseButton,
    EventField, ScrollEventUnit,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

/// Create a CGEventSource for input events
fn create_event_source() -> Result<CGEventSource> {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .ok_or_else(|| DesktopCliError::AutomationError("Failed to create CGEventSource".to_string()))
}

/// Post an event to the HID system
fn post_event(event: &CGEvent) -> Result<()> {
    event.post(CGEventTapLocation::HID);
    Ok(())
}

/// Move the mouse to the specified screen coordinates
fn move_mouse(x: f64, y: f64) -> Result<()> {
    let source = create_event_source()?;
    let point = CGPoint::new(x, y);

    let event = CGEvent::new_mouse_event(
        source,
        CGEventType::MouseMoved,
        point,
        CGMouseButton::Left,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create mouse move event".to_string()))?;

    post_event(&event)
}

/// Click at the specified window-relative coordinates
pub fn click_at_coords(window: WindowId, window_x: i32, window_y: i32) -> Result<()> {
    // Convert window-relative to screen coordinates
    let (screen_x, screen_y) = window_to_screen_coords(window, window_x, window_y)?;

    click_impl(screen_x, screen_y)?;
    tracing::debug!(
        "Clicked at window coords ({}, {}), screen coords ({}, {})",
        window_x,
        window_y,
        screen_x,
        screen_y
    );

    Ok(())
}

/// Click at absolute screen coordinates (no window offset conversion)
/// Use this when coordinates are already in screen space (e.g., from AXUIElement position)
pub fn click_at_screen_coords(screen_x: i32, screen_y: i32) -> Result<()> {
    click_impl(screen_x, screen_y)?;
    tracing::debug!("Clicked at screen coords ({}, {})", screen_x, screen_y);
    Ok(())
}

fn click_impl(screen_x: i32, screen_y: i32) -> Result<()> {
    let source = create_event_source()?;
    let point = CGPoint::new(screen_x as f64, screen_y as f64);

    // Move to position
    let move_event = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::MouseMoved,
        point,
        CGMouseButton::Left,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create move event".to_string()))?;
    post_event(&move_event)?;

    sleep(Duration::from_millis(10));

    // Mouse down
    let down_event = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::LeftMouseDown,
        point,
        CGMouseButton::Left,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create mouse down event".to_string()))?;
    post_event(&down_event)?;

    sleep(Duration::from_millis(10));

    // Mouse up
    let up_event = CGEvent::new_mouse_event(
        source,
        CGEventType::LeftMouseUp,
        point,
        CGMouseButton::Left,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create mouse up event".to_string()))?;
    post_event(&up_event)?;

    Ok(())
}

/// Double-click at the specified window-relative coordinates
pub fn double_click_at_coords(window: WindowId, window_x: i32, window_y: i32) -> Result<()> {
    let (screen_x, screen_y) = window_to_screen_coords(window, window_x, window_y)?;

    double_click_impl(screen_x, screen_y)?;
    tracing::debug!("Double-clicked at window coords ({}, {})", window_x, window_y);

    Ok(())
}

/// Double-click at absolute screen coordinates
pub fn double_click_at_screen_coords(screen_x: i32, screen_y: i32) -> Result<()> {
    double_click_impl(screen_x, screen_y)?;
    tracing::debug!("Double-clicked at screen coords ({}, {})", screen_x, screen_y);
    Ok(())
}

fn double_click_impl(screen_x: i32, screen_y: i32) -> Result<()> {
    let source = create_event_source()?;
    let point = CGPoint::new(screen_x as f64, screen_y as f64);

    // First click
    let down1 = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::LeftMouseDown,
        point,
        CGMouseButton::Left,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create event".to_string()))?;
    down1.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 1);
    post_event(&down1)?;

    let up1 = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::LeftMouseUp,
        point,
        CGMouseButton::Left,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create event".to_string()))?;
    up1.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 1);
    post_event(&up1)?;

    sleep(Duration::from_millis(50));

    // Second click (with click count = 2)
    let down2 = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::LeftMouseDown,
        point,
        CGMouseButton::Left,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create event".to_string()))?;
    down2.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 2);
    post_event(&down2)?;

    let up2 = CGEvent::new_mouse_event(
        source,
        CGEventType::LeftMouseUp,
        point,
        CGMouseButton::Left,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create event".to_string()))?;
    up2.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, 2);
    post_event(&up2)?;

    Ok(())
}

/// Right-click at the specified window-relative coordinates
pub fn right_click_at_coords(window: WindowId, window_x: i32, window_y: i32) -> Result<()> {
    let (screen_x, screen_y) = window_to_screen_coords(window, window_x, window_y)?;

    right_click_impl(screen_x, screen_y)?;
    tracing::debug!("Right-clicked at window coords ({}, {})", window_x, window_y);

    Ok(())
}

/// Right-click at absolute screen coordinates
pub fn right_click_at_screen_coords(screen_x: i32, screen_y: i32) -> Result<()> {
    right_click_impl(screen_x, screen_y)?;
    tracing::debug!("Right-clicked at screen coords ({}, {})", screen_x, screen_y);
    Ok(())
}

fn right_click_impl(screen_x: i32, screen_y: i32) -> Result<()> {
    let source = create_event_source()?;
    let point = CGPoint::new(screen_x as f64, screen_y as f64);

    // Move to position
    let move_event = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::MouseMoved,
        point,
        CGMouseButton::Right,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create event".to_string()))?;
    post_event(&move_event)?;

    sleep(Duration::from_millis(10));

    // Right mouse down
    let down_event = CGEvent::new_mouse_event(
        source.clone(),
        CGEventType::RightMouseDown,
        point,
        CGMouseButton::Right,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create event".to_string()))?;
    post_event(&down_event)?;

    sleep(Duration::from_millis(10));

    // Right mouse up
    let up_event = CGEvent::new_mouse_event(
        source,
        CGEventType::RightMouseUp,
        point,
        CGMouseButton::Right,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create event".to_string()))?;
    post_event(&up_event)?;

    Ok(())
}

/// Scroll at the current mouse position
/// amount: positive = scroll up, negative = scroll down
pub fn scroll(amount: i32) -> Result<()> {
    let source = create_event_source()?;

    // Create scroll wheel event
    let event = CGEvent::new_scroll_event(
        source,
        ScrollEventUnit::LINE,
        1, // wheel count
        amount,
        0,
        0,
    )
    .ok_or_else(|| DesktopCliError::AutomationError("Failed to create scroll event".to_string()))?;

    post_event(&event)?;

    tracing::debug!("Scrolled by {}", amount);

    Ok(())
}

/// Type text at the current cursor position
pub fn type_text(text: &str) -> Result<()> {
    let source = create_event_source()?;

    for ch in text.chars() {
        // For each character, we need to find the keycode and modifiers
        if let Some((keycode, modifiers)) = char_to_keycode(ch) {
            // Key down with modifiers
            let down = CGEvent::new_keyboard_event(source.clone(), keycode, true)
                .ok_or_else(|| {
                    DesktopCliError::AutomationError("Failed to create key event".to_string())
                })?;
            down.set_flags(modifiers);
            post_event(&down)?;

            // Key up
            let up = CGEvent::new_keyboard_event(source.clone(), keycode, false)
                .ok_or_else(|| {
                    DesktopCliError::AutomationError("Failed to create key event".to_string())
                })?;
            up.set_flags(modifiers);
            post_event(&up)?;

            sleep(Duration::from_millis(5));
        } else {
            // Use Unicode input for unsupported characters
            // CGEventKeyboardSetUnicodeString would be ideal but not exposed
            tracing::warn!("No keycode for character: {}", ch);
        }
    }

    tracing::debug!("Typed text: {}", text);

    Ok(())
}

/// Convert character to macOS keycode and modifiers
fn char_to_keycode(ch: char) -> Option<(CGKeyCode, CGEventFlags)> {
    // macOS virtual key codes
    // See: https://developer.apple.com/documentation/coregraphics/cgkeycode
    let no_mod = CGEventFlags::CGEventFlagNull;
    let shift = CGEventFlags::CGEventFlagShift;

    let result = match ch {
        'a' => Some((0x00, no_mod)),
        'b' => Some((0x0B, no_mod)),
        'c' => Some((0x08, no_mod)),
        'd' => Some((0x02, no_mod)),
        'e' => Some((0x0E, no_mod)),
        'f' => Some((0x03, no_mod)),
        'g' => Some((0x05, no_mod)),
        'h' => Some((0x04, no_mod)),
        'i' => Some((0x22, no_mod)),
        'j' => Some((0x26, no_mod)),
        'k' => Some((0x28, no_mod)),
        'l' => Some((0x25, no_mod)),
        'm' => Some((0x2E, no_mod)),
        'n' => Some((0x2D, no_mod)),
        'o' => Some((0x1F, no_mod)),
        'p' => Some((0x23, no_mod)),
        'q' => Some((0x0C, no_mod)),
        'r' => Some((0x0F, no_mod)),
        's' => Some((0x01, no_mod)),
        't' => Some((0x11, no_mod)),
        'u' => Some((0x20, no_mod)),
        'v' => Some((0x09, no_mod)),
        'w' => Some((0x0D, no_mod)),
        'x' => Some((0x07, no_mod)),
        'y' => Some((0x10, no_mod)),
        'z' => Some((0x06, no_mod)),
        'A' => Some((0x00, shift)),
        'B' => Some((0x0B, shift)),
        'C' => Some((0x08, shift)),
        'D' => Some((0x02, shift)),
        'E' => Some((0x0E, shift)),
        'F' => Some((0x03, shift)),
        'G' => Some((0x05, shift)),
        'H' => Some((0x04, shift)),
        'I' => Some((0x22, shift)),
        'J' => Some((0x26, shift)),
        'K' => Some((0x28, shift)),
        'L' => Some((0x25, shift)),
        'M' => Some((0x2E, shift)),
        'N' => Some((0x2D, shift)),
        'O' => Some((0x1F, shift)),
        'P' => Some((0x23, shift)),
        'Q' => Some((0x0C, shift)),
        'R' => Some((0x0F, shift)),
        'S' => Some((0x01, shift)),
        'T' => Some((0x11, shift)),
        'U' => Some((0x20, shift)),
        'V' => Some((0x09, shift)),
        'W' => Some((0x0D, shift)),
        'X' => Some((0x07, shift)),
        'Y' => Some((0x10, shift)),
        'Z' => Some((0x06, shift)),
        '0' => Some((0x1D, no_mod)),
        '1' => Some((0x12, no_mod)),
        '2' => Some((0x13, no_mod)),
        '3' => Some((0x14, no_mod)),
        '4' => Some((0x15, no_mod)),
        '5' => Some((0x17, no_mod)),
        '6' => Some((0x16, no_mod)),
        '7' => Some((0x1A, no_mod)),
        '8' => Some((0x1C, no_mod)),
        '9' => Some((0x19, no_mod)),
        ' ' => Some((0x31, no_mod)),
        '\n' => Some((0x24, no_mod)),
        '\t' => Some((0x30, no_mod)),
        '-' => Some((0x1B, no_mod)),
        '=' => Some((0x18, no_mod)),
        '[' => Some((0x21, no_mod)),
        ']' => Some((0x1E, no_mod)),
        '\\' => Some((0x2A, no_mod)),
        ';' => Some((0x29, no_mod)),
        '\'' => Some((0x27, no_mod)),
        '`' => Some((0x32, no_mod)),
        ',' => Some((0x2B, no_mod)),
        '.' => Some((0x2F, no_mod)),
        '/' => Some((0x2C, no_mod)),
        '!' => Some((0x12, shift)),
        '@' => Some((0x13, shift)),
        '#' => Some((0x14, shift)),
        '$' => Some((0x15, shift)),
        '%' => Some((0x17, shift)),
        '^' => Some((0x16, shift)),
        '&' => Some((0x1A, shift)),
        '*' => Some((0x1C, shift)),
        '(' => Some((0x19, shift)),
        ')' => Some((0x1D, shift)),
        '_' => Some((0x1B, shift)),
        '+' => Some((0x18, shift)),
        '{' => Some((0x21, shift)),
        '}' => Some((0x1E, shift)),
        '|' => Some((0x2A, shift)),
        ':' => Some((0x29, shift)),
        '"' => Some((0x27, shift)),
        '~' => Some((0x32, shift)),
        '<' => Some((0x2B, shift)),
        '>' => Some((0x2F, shift)),
        '?' => Some((0x2C, shift)),
        _ => None,
    };

    result
}

/// Get keycode for named key
fn get_named_keycode(key_name: &str) -> Option<(CGKeyCode, CGEventFlags)> {
    let no_mod = CGEventFlags::CGEventFlagNull;
    let ctrl = CGEventFlags::CGEventFlagControl;
    let alt = CGEventFlags::CGEventFlagAlternate;
    let cmd = CGEventFlags::CGEventFlagCommand;
    let shift_flag = CGEventFlags::CGEventFlagShift;

    match key_name.to_lowercase().as_str() {
        // Modifiers (return None as they're handled differently)
        "ctrl" | "control" => Some((0x3B, no_mod)), // Control key
        "alt" | "option" => Some((0x3A, no_mod)),   // Option key
        "cmd" | "command" | "win" | "super" => Some((0x37, no_mod)), // Command key
        "shift" => Some((0x38, no_mod)),            // Shift key
        // Special keys
        "enter" | "return" => Some((0x24, no_mod)),
        "tab" => Some((0x30, no_mod)),
        "space" | "spacebar" => Some((0x31, no_mod)),
        "backspace" | "back" | "delete" => Some((0x33, no_mod)),
        "escape" | "esc" => Some((0x35, no_mod)),
        "capslock" | "caps" => Some((0x39, no_mod)),
        // Function keys
        "f1" => Some((0x7A, no_mod)),
        "f2" => Some((0x78, no_mod)),
        "f3" => Some((0x63, no_mod)),
        "f4" => Some((0x76, no_mod)),
        "f5" => Some((0x60, no_mod)),
        "f6" => Some((0x61, no_mod)),
        "f7" => Some((0x62, no_mod)),
        "f8" => Some((0x64, no_mod)),
        "f9" => Some((0x65, no_mod)),
        "f10" => Some((0x6D, no_mod)),
        "f11" => Some((0x67, no_mod)),
        "f12" => Some((0x6F, no_mod)),
        // Arrow keys
        "up" => Some((0x7E, no_mod)),
        "down" => Some((0x7D, no_mod)),
        "left" => Some((0x7B, no_mod)),
        "right" => Some((0x7C, no_mod)),
        // Navigation keys
        "home" => Some((0x73, no_mod)),
        "end" => Some((0x77, no_mod)),
        "pageup" | "pgup" => Some((0x74, no_mod)),
        "pagedown" | "pgdn" => Some((0x79, no_mod)),
        "del" | "forwarddelete" => Some((0x75, no_mod)),
        // Single letters (for key combos)
        "a" => Some((0x00, no_mod)),
        "b" => Some((0x0B, no_mod)),
        "c" => Some((0x08, no_mod)),
        "d" => Some((0x02, no_mod)),
        "e" => Some((0x0E, no_mod)),
        "f" => Some((0x03, no_mod)),
        "g" => Some((0x05, no_mod)),
        "h" => Some((0x04, no_mod)),
        "i" => Some((0x22, no_mod)),
        "j" => Some((0x26, no_mod)),
        "k" => Some((0x28, no_mod)),
        "l" => Some((0x25, no_mod)),
        "m" => Some((0x2E, no_mod)),
        "n" => Some((0x2D, no_mod)),
        "o" => Some((0x1F, no_mod)),
        "p" => Some((0x23, no_mod)),
        "q" => Some((0x0C, no_mod)),
        "r" => Some((0x0F, no_mod)),
        "s" => Some((0x01, no_mod)),
        "t" => Some((0x11, no_mod)),
        "u" => Some((0x20, no_mod)),
        "v" => Some((0x09, no_mod)),
        "w" => Some((0x0D, no_mod)),
        "x" => Some((0x07, no_mod)),
        "y" => Some((0x10, no_mod)),
        "z" => Some((0x06, no_mod)),
        // Numbers
        "0" => Some((0x1D, no_mod)),
        "1" => Some((0x12, no_mod)),
        "2" => Some((0x13, no_mod)),
        "3" => Some((0x14, no_mod)),
        "4" => Some((0x15, no_mod)),
        "5" => Some((0x17, no_mod)),
        "6" => Some((0x16, no_mod)),
        "7" => Some((0x1A, no_mod)),
        "8" => Some((0x1C, no_mod)),
        "9" => Some((0x19, no_mod)),
        _ => None,
    }
}

/// Send a key combination (e.g., "cmd+c", "ctrl+alt+delete", "cmd+shift+s")
pub fn send_keys(keys: &str) -> Result<()> {
    let source = create_event_source()?;
    let parts: Vec<&str> = keys.split('+').map(|s| s.trim()).collect();

    if parts.is_empty() {
        return Err(DesktopCliError::AutomationError(
            "Empty key combination".to_string(),
        ));
    }

    // Separate modifiers from the final key
    let mut flags = CGEventFlags::CGEventFlagNull;
    let mut final_keycode: Option<CGKeyCode> = None;

    for (i, part) in parts.iter().enumerate() {
        let part_lower = part.to_lowercase();

        // Check if it's a modifier
        match part_lower.as_str() {
            "ctrl" | "control" => {
                flags |= CGEventFlags::CGEventFlagControl;
            }
            "alt" | "option" => {
                flags |= CGEventFlags::CGEventFlagAlternate;
            }
            "cmd" | "command" | "win" | "super" => {
                flags |= CGEventFlags::CGEventFlagCommand;
            }
            "shift" => {
                flags |= CGEventFlags::CGEventFlagShift;
            }
            _ => {
                // Not a modifier, must be the final key
                if let Some((keycode, _)) = get_named_keycode(part) {
                    final_keycode = Some(keycode);
                } else {
                    return Err(DesktopCliError::AutomationError(format!(
                        "Unknown key: '{}'. Use names like: cmd, ctrl, alt, shift, enter, tab, esc, f1-f12, a-z, 0-9",
                        part
                    )));
                }
            }
        }
    }

    let keycode = final_keycode.ok_or_else(|| {
        DesktopCliError::AutomationError(
            "No main key specified in combination".to_string(),
        )
    })?;

    // Key down with modifiers
    let down = CGEvent::new_keyboard_event(source.clone(), keycode, true)
        .ok_or_else(|| DesktopCliError::AutomationError("Failed to create key event".to_string()))?;
    down.set_flags(flags);
    post_event(&down)?;

    sleep(Duration::from_millis(10));

    // Key up
    let up = CGEvent::new_keyboard_event(source, keycode, false)
        .ok_or_else(|| DesktopCliError::AutomationError("Failed to create key event".to_string()))?;
    up.set_flags(flags);
    post_event(&up)?;

    tracing::debug!("Sent key combination: {}", keys);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_keycode() {
        assert!(char_to_keycode('a').is_some());
        assert!(char_to_keycode('A').is_some());
        assert!(char_to_keycode('1').is_some());
        assert!(char_to_keycode(' ').is_some());
    }

    #[test]
    fn test_get_named_keycode() {
        assert!(get_named_keycode("enter").is_some());
        assert!(get_named_keycode("ctrl").is_some());
        assert!(get_named_keycode("f1").is_some());
        assert!(get_named_keycode("invalid").is_none());
    }
}
