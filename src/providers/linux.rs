use crate::semantic::{CapabilityKind, RelationKind, Role, SemanticAction, State};

pub fn role_from_atspi(role: &str) -> Role {
    let r = role.to_ascii_lowercase().replace('_', " ");
    match r.as_str() {
        "push button" | "button" => Role::Button,
        "check box" => Role::Checkbox,
        "radio button" => Role::RadioButton,
        "text" | "entry" => Role::TextInput,
        "password text" => Role::PasswordInput,
        "menu" => Role::Menu,
        "menu bar" => Role::MenuBar,
        "menu item" | "check menu item" | "radio menu item" => Role::MenuItem,
        "tool bar" => Role::Toolbar,
        "page tab list" => Role::TabList,
        "page tab" => Role::Tab,
        "list" => Role::List,
        "list item" => Role::ListItem,
        "tree" | "tree table" => Role::Tree,
        "tree item" => Role::TreeItem,
        "table" => Role::Table,
        "table cell" => Role::Cell,
        "document frame" | "document text" | "document web" => Role::Document,
        "canvas" => Role::Canvas,
        "dialog" | "file chooser" => Role::Dialog,
        "alert" => Role::Alert,
        "status bar" => Role::StatusBar,
        "tool tip" => Role::Tooltip,
        "panel" | "filler" => Role::Panel,
        "application" => Role::Application,
        "frame" | "window" => Role::Window,
        _ => Role::parse(role),
    }
}

pub fn state_from_atspi(state: &str) -> Option<State> {
    State::parse(&state.to_ascii_lowercase().replace('_', "-"))
}

pub fn capability_from_interface(interface: &str) -> Option<CapabilityKind> {
    Some(match interface.to_ascii_lowercase().as_str() {
        "action" => CapabilityKind::Action,
        "value" => CapabilityKind::Value,
        "text" => CapabilityKind::Text,
        "editabletext" | "editable-text" => CapabilityKind::EditableText,
        "selection" => CapabilityKind::Selection,
        "table" => CapabilityKind::Table,
        "tablecell" | "table-cell" => CapabilityKind::TableCell,
        "document" => CapabilityKind::Document,
        "component" => CapabilityKind::Component,
        "hypertext" => CapabilityKind::Hypertext,
        "image" => CapabilityKind::Image,
        _ => return None,
    })
}

pub fn relation_from_atspi(name: &str) -> RelationKind {
    match name.to_ascii_lowercase().replace('_', "-").as_str() {
        "label-for" => RelationKind::LabelFor,
        "labelled-by" => RelationKind::LabelledBy,
        "controller-for" => RelationKind::Controls,
        "controlled-by" => RelationKind::ControlledBy,
        "member-of" => RelationKind::MemberOf,
        "tooltip-for" => RelationKind::TooltipFor,
        "flows-to" => RelationKind::FlowsTo,
        "flows-from" => RelationKind::FlowsFrom,
        "subwindow-of" => RelationKind::SubwindowOf,
        "embeds" => RelationKind::Embeds,
        "embedded-by" => RelationKind::EmbeddedBy,
        "described-by" => RelationKind::DescribedBy,
        "description-for" => RelationKind::DescriptionFor,
        "details" => RelationKind::Details,
        "details-for" => RelationKind::DetailsFor,
        "error-for" => RelationKind::ErrorFor,
        "error-message" => RelationKind::ErrorMessage,
        "popup-for" => RelationKind::PopupFor,
        other => RelationKind::Native(other.into()),
    }
}

pub fn semantic_action_from_name(name: &str) -> Option<SemanticAction> {
    match name.to_ascii_lowercase().replace('_', "-").as_str() {
        "click" | "press" | "activate" | "open" => Some(SemanticAction::Activate),
        "toggle" => Some(SemanticAction::Toggle),
        "select" => Some(SemanticAction::Select),
        "expand" => Some(SemanticAction::Expand),
        "collapse" => Some(SemanticAction::Collapse),
        "show-menu" => Some(SemanticAction::ShowMenu),
        "increment" => Some(SemanticAction::Increment),
        "decrement" => Some(SemanticAction::Decrement),
        _ => None,
    }
}
