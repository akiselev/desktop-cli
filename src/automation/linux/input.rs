//! Input simulation via enigo

use crate::error::{DesktopCliError, Result};
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};

pub fn click_at_coords(x: i32, y: i32) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        DesktopCliError::AutomationError(format!("Failed to create enigo instance: {}", e))
    })?;

    enigo
        .move_mouse(x, y, Coordinate::Abs)
        .map_err(|e| DesktopCliError::AutomationError(format!("Failed to move mouse: {}", e)))?;

    enigo
        .button(Button::Left, Direction::Click)
        .map_err(|e| DesktopCliError::AutomationError(format!("Failed to click: {}", e)))?;

    Ok(())
}

pub fn type_text(text: &str) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        DesktopCliError::AutomationError(format!("Failed to create enigo instance: {}", e))
    })?;

    enigo
        .text(text)
        .map_err(|e| DesktopCliError::AutomationError(format!("Failed to type text: {}", e)))?;

    Ok(())
}

pub fn send_keys(combo: &str) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        DesktopCliError::AutomationError(format!("Failed to create enigo instance: {}", e))
    })?;

    let keys: Vec<&str> = combo.split('+').map(|s| s.trim()).collect();

    let mut modifiers = Vec::new();
    let mut main_key = None;

    for key_str in &keys {
        let key_lower = key_str.to_lowercase();
        match key_lower.as_str() {
            "ctrl" | "control" => modifiers.push(Key::Control),
            "alt" => modifiers.push(Key::Alt),
            "shift" => modifiers.push(Key::Shift),
            "meta" | "super" | "win" | "cmd" => modifiers.push(Key::Meta),
            _ => {
                main_key = Some(parse_key(key_str)?);
            }
        }
    }

    for modifier in &modifiers {
        enigo.key(*modifier, Direction::Press).map_err(|e| {
            DesktopCliError::AutomationError(format!("Failed to press modifier: {}", e))
        })?;
    }

    if let Some(key) = main_key {
        enigo
            .key(key, Direction::Click)
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to press key: {}", e)))?;
    }

    for modifier in modifiers.iter().rev() {
        enigo.key(*modifier, Direction::Release).map_err(|e| {
            DesktopCliError::AutomationError(format!("Failed to release modifier: {}", e))
        })?;
    }

    Ok(())
}

pub fn scroll(direction: &str, amount: i32) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        DesktopCliError::AutomationError(format!("Failed to create enigo instance: {}", e))
    })?;

    let scroll_amount = match direction.to_lowercase().as_str() {
        "up" => amount,
        "down" => -amount,
        _ => {
            return Err(DesktopCliError::AutomationError(format!(
                "Invalid scroll direction: {}",
                direction
            )))
        }
    };

    enigo
        .scroll(scroll_amount, enigo::Axis::Vertical)
        .map_err(|e| DesktopCliError::AutomationError(format!("Failed to scroll: {}", e)))?;

    Ok(())
}

fn parse_key(key_str: &str) -> Result<Key> {
    let key_lower = key_str.to_lowercase();

    match key_lower.as_str() {
        "a" => Ok(Key::Unicode('a')),
        "b" => Ok(Key::Unicode('b')),
        "c" => Ok(Key::Unicode('c')),
        "d" => Ok(Key::Unicode('d')),
        "e" => Ok(Key::Unicode('e')),
        "f" => Ok(Key::Unicode('f')),
        "g" => Ok(Key::Unicode('g')),
        "h" => Ok(Key::Unicode('h')),
        "i" => Ok(Key::Unicode('i')),
        "j" => Ok(Key::Unicode('j')),
        "k" => Ok(Key::Unicode('k')),
        "l" => Ok(Key::Unicode('l')),
        "m" => Ok(Key::Unicode('m')),
        "n" => Ok(Key::Unicode('n')),
        "o" => Ok(Key::Unicode('o')),
        "p" => Ok(Key::Unicode('p')),
        "q" => Ok(Key::Unicode('q')),
        "r" => Ok(Key::Unicode('r')),
        "s" => Ok(Key::Unicode('s')),
        "t" => Ok(Key::Unicode('t')),
        "u" => Ok(Key::Unicode('u')),
        "v" => Ok(Key::Unicode('v')),
        "w" => Ok(Key::Unicode('w')),
        "x" => Ok(Key::Unicode('x')),
        "y" => Ok(Key::Unicode('y')),
        "z" => Ok(Key::Unicode('z')),
        "enter" | "return" => Ok(Key::Return),
        "space" => Ok(Key::Space),
        "tab" => Ok(Key::Tab),
        "escape" | "esc" => Ok(Key::Escape),
        "backspace" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "up" | "uparrow" => Ok(Key::UpArrow),
        "down" | "downarrow" => Ok(Key::DownArrow),
        "left" | "leftarrow" => Ok(Key::LeftArrow),
        "right" | "rightarrow" => Ok(Key::RightArrow),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" => Ok(Key::PageUp),
        "pagedown" => Ok(Key::PageDown),
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),
        _ => {
            if key_str.len() == 1 {
                Ok(Key::Unicode(key_str.chars().next().unwrap()))
            } else {
                Err(DesktopCliError::AutomationError(format!(
                    "Unknown key: {}",
                    key_str
                )))
            }
        }
    }
}
