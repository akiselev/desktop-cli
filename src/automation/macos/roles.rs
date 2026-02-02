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
        "AXGroup" => "Group",
        "AXScrollBar" => "ScrollBar",
        "AXTable" => "Table",
        "AXCell" => "DataItem",
        "AXImage" => "Image",
        "AXTextArea" => "Edit",
        "AXToolbar" => "ToolBar",
        "AXTabGroup" => "Tab",
        "AXScrollArea" => "Pane",
        "AXSplitGroup" => "Pane",
        _ => "Custom",
    }
    .to_string()
}
