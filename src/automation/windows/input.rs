use crate::automation::windows::coordinates::{normalize_screen_coords, window_to_screen_coords};
use crate::error::{DesktopCliError, Result};
use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
    MOUSEINPUT, VIRTUAL_KEY,
};

/// Click at the specified window-relative coordinates
pub fn click_at_coords(hwnd: HWND, window_x: i32, window_y: i32) -> Result<()> {
    // Convert window-relative to screen coordinates
    let (screen_x, screen_y) = window_to_screen_coords(hwnd, window_x, window_y)?;

    // Normalize to 0-65535 range for SendInput
    let (normalized_x, normalized_y) = normalize_screen_coords(screen_x, screen_y)?;

    let inputs = [
        // Move mouse to position
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // Mouse button down
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // Mouse button up
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    unsafe {
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err(DesktopCliError::AutomationError(format!(
                "SendInput failed: sent {} out of {} events",
                sent,
                inputs.len()
            )));
        }
    }

    tracing::debug!("Clicked at window coords ({}, {}), screen coords ({}, {})",
        window_x, window_y, screen_x, screen_y);

    Ok(())
}

/// Double-click at the specified window-relative coordinates
pub fn double_click_at_coords(hwnd: HWND, window_x: i32, window_y: i32) -> Result<()> {
    // Convert window-relative to screen coordinates
    let (screen_x, screen_y) = window_to_screen_coords(hwnd, window_x, window_y)?;

    // Normalize to 0-65535 range for SendInput
    let (normalized_x, normalized_y) = normalize_screen_coords(screen_x, screen_y)?;

    // First click
    let click1 = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    unsafe {
        SendInput(&click1, std::mem::size_of::<INPUT>() as i32);
    }

    // Small delay between clicks (Windows double-click timing)
    sleep(Duration::from_millis(50));

    // Second click
    let click2 = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    unsafe {
        let sent = SendInput(&click2, std::mem::size_of::<INPUT>() as i32);
        if sent != click2.len() as u32 {
            return Err(DesktopCliError::AutomationError(format!(
                "SendInput failed for double-click: sent {} out of {} events",
                sent,
                click2.len()
            )));
        }
    }

    tracing::debug!("Double-clicked at window coords ({}, {})", window_x, window_y);

    Ok(())
}

/// Right-click at the specified window-relative coordinates
pub fn right_click_at_coords(hwnd: HWND, window_x: i32, window_y: i32) -> Result<()> {
    let (screen_x, screen_y) = window_to_screen_coords(hwnd, window_x, window_y)?;
    let (normalized_x, normalized_y) = normalize_screen_coords(screen_x, screen_y)?;

    let inputs = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_RIGHTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: normalized_x,
                    dy: normalized_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_RIGHTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    unsafe {
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err(DesktopCliError::AutomationError(format!(
                "SendInput failed for right-click: sent {} out of {} events",
                sent,
                inputs.len()
            )));
        }
    }

    tracing::debug!("Right-clicked at window coords ({}, {})", window_x, window_y);

    Ok(())
}

/// Scroll at the current mouse position
/// amount: positive = scroll up, negative = scroll down (in wheel delta units, 120 = one notch)
pub fn scroll(amount: i32) -> Result<()> {
    let inputs = [INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: amount as u32,
                dwFlags: MOUSEEVENTF_WHEEL,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }];

    unsafe {
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err(DesktopCliError::AutomationError(format!(
                "SendInput failed for scroll: sent {} out of {} events",
                sent,
                inputs.len()
            )));
        }
    }

    tracing::debug!("Scrolled by {}", amount);

    Ok(())
}

/// Type text at the current cursor position
pub fn type_text(text: &str) -> Result<()> {
    let mut inputs = Vec::new();

    for ch in text.chars() {
        // Key down
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: ch as u16,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });

        // Key up
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: ch as u16,
                    dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    if !inputs.is_empty() {
        unsafe {
            let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            if sent != inputs.len() as u32 {
                return Err(DesktopCliError::AutomationError(format!(
                    "SendInput failed: sent {} out of {} events",
                    sent,
                    inputs.len()
                )));
            }
        }
    }

    tracing::debug!("Typed text: {}", text);

    Ok(())
}

/// Press a specific key (using virtual key code)
pub fn press_key(vk_code: u16) -> Result<()> {
    let inputs = [
        // Key down
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk_code),
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        // Key up
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk_code),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    unsafe {
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err(DesktopCliError::AutomationError(format!(
                "SendInput failed: sent {} out of {} events",
                sent,
                inputs.len()
            )));
        }
    }

    tracing::debug!("Pressed key with VK code: {}", vk_code);

    Ok(())
}

/// Virtual key codes for common keys
pub fn get_vk_code(key_name: &str) -> Option<u16> {
    let key_map: HashMap<&str, u16> = [
        // Letters
        ("a", 0x41), ("b", 0x42), ("c", 0x43), ("d", 0x44), ("e", 0x45),
        ("f", 0x46), ("g", 0x47), ("h", 0x48), ("i", 0x49), ("j", 0x4A),
        ("k", 0x4B), ("l", 0x4C), ("m", 0x4D), ("n", 0x4E), ("o", 0x4F),
        ("p", 0x50), ("q", 0x51), ("r", 0x52), ("s", 0x53), ("t", 0x54),
        ("u", 0x55), ("v", 0x56), ("w", 0x57), ("x", 0x58), ("y", 0x59),
        ("z", 0x5A),
        // Numbers
        ("0", 0x30), ("1", 0x31), ("2", 0x32), ("3", 0x33), ("4", 0x34),
        ("5", 0x35), ("6", 0x36), ("7", 0x37), ("8", 0x38), ("9", 0x39),
        // Function keys
        ("f1", 0x70), ("f2", 0x71), ("f3", 0x72), ("f4", 0x73), ("f5", 0x74),
        ("f6", 0x75), ("f7", 0x76), ("f8", 0x77), ("f9", 0x78), ("f10", 0x79),
        ("f11", 0x7A), ("f12", 0x7B),
        // Modifiers
        ("ctrl", 0x11), ("control", 0x11), ("lctrl", 0xA2), ("rctrl", 0xA3),
        ("alt", 0x12), ("menu", 0x12), ("lalt", 0xA4), ("ralt", 0xA5),
        ("shift", 0x10), ("lshift", 0xA0), ("rshift", 0xA1),
        ("win", 0x5B), ("lwin", 0x5B), ("rwin", 0x5C),
        // Special keys
        ("enter", 0x0D), ("return", 0x0D),
        ("tab", 0x09),
        ("escape", 0x1B), ("esc", 0x1B),
        ("space", 0x20), ("spacebar", 0x20),
        ("backspace", 0x08), ("back", 0x08),
        ("delete", 0x2E), ("del", 0x2E),
        ("insert", 0x2D), ("ins", 0x2D),
        ("home", 0x24),
        ("end", 0x23),
        ("pageup", 0x21), ("pgup", 0x21),
        ("pagedown", 0x22), ("pgdn", 0x22),
        // Arrow keys
        ("up", 0x26), ("down", 0x28), ("left", 0x25), ("right", 0x27),
        // Other
        ("printscreen", 0x2C), ("prtsc", 0x2C),
        ("pause", 0x13),
        ("capslock", 0x14), ("caps", 0x14),
        ("numlock", 0x90),
        ("scrolllock", 0x91),
    ].iter().cloned().collect();

    key_map.get(key_name.to_lowercase().as_str()).copied()
}

/// Send a key combination (e.g., "ctrl+c", "alt+f4", "ctrl+shift+s")
pub fn send_keys(keys: &str) -> Result<()> {
    let parts: Vec<&str> = keys.split('+').map(|s| s.trim()).collect();

    if parts.is_empty() {
        return Err(DesktopCliError::AutomationError("Empty key combination".to_string()));
    }

    // Parse all key codes
    let mut key_codes: Vec<u16> = Vec::new();
    for part in &parts {
        if let Some(vk) = get_vk_code(part) {
            key_codes.push(vk);
        } else {
            return Err(DesktopCliError::AutomationError(format!(
                "Unknown key: '{}'. Use names like: ctrl, alt, shift, enter, tab, esc, f1-f12, a-z, 0-9",
                part
            )));
        }
    }

    let mut inputs = Vec::new();

    // Press all keys down (in order)
    for &vk in &key_codes {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: Default::default(),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    // Release all keys (in reverse order)
    for &vk in key_codes.iter().rev() {
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        });
    }

    unsafe {
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err(DesktopCliError::AutomationError(format!(
                "SendInput failed for key combo: sent {} out of {} events",
                sent,
                inputs.len()
            )));
        }
    }

    tracing::debug!("Sent key combination: {}", keys);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_text_creates_inputs() {
        // This test just ensures the code compiles
        // Actual typing tests would require a GUI environment
        let _ = type_text("test");
    }
}
