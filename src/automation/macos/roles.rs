//! macOS AX role to UIA control type mapping

pub fn map_role(ax_role: &str) -> String {
    match ax_role {
        "AXButton" => "Button",
        "AXTextField" => "Edit",
        "AXStaticText" => "Text",
        "AXMenu" => "Menu",
        "AXMenuItem" => "MenuItem",
        "AXCheckBox" => "CheckBox",
        "AXRadioButton" => "RadioButton",
        "AXComboBox" => "ComboBox",
        "AXList" => "List",
        "AXRow" => "ListItem",
        "AXWindow" => "Window",
        "AXGroup" => "Pane",
        "AXScrollBar" => "ScrollBar",
        "AXTable" => "Table",
        "AXCell" => "DataItem",
        _ => "Custom",
    }.to_string()
}
