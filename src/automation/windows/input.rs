use crate::automation::windows::coordinates::{normalize_screen_coords, window_to_screen_coords};
use crate::error::{DesktopCliError, Result};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEINPUT, VIRTUAL_KEY,
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
