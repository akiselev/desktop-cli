//! AT-SPI2 role to UIA control type mapping

/// Maps AT-SPI2 role strings to Windows UIA-compatible control types
///
/// Normalizes cross-platform element trees for consistent selector syntax.
pub fn map_role(atspi_role: &str) -> String {
    match atspi_role {
        "push button" | "push-button" => "Button",
        "text" => "Edit",
        "menu" => "Menu",
        "menu item" | "menu-item" => "MenuItem",
        "check box" | "check-box" => "CheckBox",
        "radio button" | "radio-button" => "RadioButton",
        "combo box" | "combo-box" => "ComboBox",
        "list" => "List",
        "list item" | "list-item" => "ListItem",
        "window" => "Window",
        "frame" => "Pane",
        "panel" => "Pane",
        "scroll bar" | "scroll-bar" => "ScrollBar",
        "table" => "Table",
        "table cell" | "table-cell" => "DataItem",
        "label" => "Text",
        _ => "Custom",
    }
    .to_string()
}
