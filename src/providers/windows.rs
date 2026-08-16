use crate::semantic::{CapabilityKind, Role, SemanticAction, State};

pub fn role_from_uia(control_type: &str) -> Role {
    Role::parse(control_type)
}

pub fn state_property(name: &str) -> Option<State> {
    Some(match name {
        "IsEnabled" => State::Enabled,
        "HasKeyboardFocus" => State::Focused,
        "IsKeyboardFocusable" => State::Focusable,
        "IsOffscreen" => State::Offscreen,
        "IsRequiredForForm" => State::Required,
        _ => return None,
    })
}

pub fn capability_from_pattern(name: &str) -> Option<CapabilityKind> {
    Some(match name {
        "Invoke" => CapabilityKind::Action,
        "Value" => CapabilityKind::Value,
        "RangeValue" => CapabilityKind::RangeValue,
        "Toggle" => CapabilityKind::Toggle,
        "ExpandCollapse" => CapabilityKind::ExpandCollapse,
        "Text" | "Text2" => CapabilityKind::Text,
        "Selection" => CapabilityKind::Selection,
        "SelectionItem" => CapabilityKind::SelectionItem,
        "Grid" | "Table" => CapabilityKind::Table,
        "GridItem" | "TableItem" => CapabilityKind::TableCell,
        "Scroll" => CapabilityKind::Scroll,
        "ScrollItem" => CapabilityKind::ScrollItem,
        "Window" => CapabilityKind::Window,
        "Transform" | "Transform2" => CapabilityKind::Transform,
        "VirtualizedItem" => CapabilityKind::VirtualizedItem,
        "ItemContainer" => CapabilityKind::ItemContainer,
        _ => return None,
    })
}

pub fn semantic_action_from_pattern(name: &str) -> Option<SemanticAction> {
    Some(match name {
        "Invoke" => SemanticAction::Activate,
        "Value" => SemanticAction::SetValue,
        "Toggle" => SemanticAction::Toggle,
        "SelectionItem" => SemanticAction::Select,
        "ExpandCollapse" => SemanticAction::Expand,
        "ScrollItem" => SemanticAction::ScrollIntoView,
        _ => return None,
    })
}
