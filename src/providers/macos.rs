use crate::semantic::{PropertyValue, RelationKind, Role, SemanticAction, State};

#[derive(Debug, Clone, PartialEq)]
pub enum AxRawValue {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    Point(f64, f64),
    Size(f64, f64),
    Rect(f64, f64, f64, f64),
    Range(usize, usize),
    Array(Vec<AxRawValue>),
    Opaque(String),
}

pub fn property_from_ax(value: AxRawValue) -> PropertyValue {
    match value {
        AxRawValue::Null => PropertyValue::Null,
        AxRawValue::Bool(v) => PropertyValue::Bool(v),
        AxRawValue::I64(v) => PropertyValue::I64(v),
        AxRawValue::F64(v) => PropertyValue::F64(v),
        AxRawValue::String(v) => PropertyValue::String(v),
        AxRawValue::Point(x, y) => PropertyValue::Point(crate::semantic::Point { x, y }),
        AxRawValue::Size(width, height) => {
            PropertyValue::Size(crate::semantic::Size { width, height })
        }
        AxRawValue::Rect(x, y, width, height) => {
            PropertyValue::Rect(crate::semantic::Rect { x, y, width, height })
        }
        AxRawValue::Range(start, end) => {
            PropertyValue::Range(crate::semantic::TextRange { start, end })
        }
        AxRawValue::Array(v) => {
            PropertyValue::Array(v.into_iter().map(property_from_ax).collect())
        }
        AxRawValue::Opaque(type_name) => PropertyValue::Opaque {
            type_name,
            debug: None,
        },
    }
}

pub fn role_from_ax(role: &str, subrole: Option<&str>) -> Role {
    if let Some("AXSearchField") = subrole {
        return Role::SearchInput;
    }
    match role {
        "AXApplication" => Role::Application,
        "AXWindow" | "AXSheet" => Role::Window,
        "AXDialog" => Role::Dialog,
        "AXButton" => Role::Button,
        "AXCheckBox" => Role::Checkbox,
        "AXRadioButton" => Role::RadioButton,
        "AXTextField" | "AXTextArea" => Role::TextInput,
        "AXStaticText" => Role::StaticText,
        "AXLink" => Role::Link,
        "AXImage" => Role::Image,
        "AXMenuBar" => Role::MenuBar,
        "AXMenu" => Role::Menu,
        "AXMenuItem" => Role::MenuItem,
        "AXToolbar" => Role::Toolbar,
        "AXTabGroup" => Role::TabList,
        "AXList" => Role::List,
        "AXRow" => Role::Row,
        "AXTable" | "AXOutline" => Role::Table,
        "AXGroup" | "AXSplitGroup" => Role::Group,
        "AXScrollArea" => Role::Panel,
        "AXSlider" => Role::Slider,
        "AXIncrementor" => Role::SpinButton,
        "AXProgressIndicator" => Role::Progress,
        "AXDocument" => Role::Document,
        _ => Role::Unknown,
    }
}

pub fn state_from_ax_attribute(attribute: &str) -> Option<State> {
    Some(match attribute {
        "AXEnabled" => State::Enabled,
        "AXFocused" => State::Focused,
        "AXSelected" => State::Selected,
        "AXExpanded" => State::Expanded,
        "AXRequired" => State::Required,
        "AXVisited" => State::Visited,
        "AXProtectedContent" => State::Protected,
        "AXModal" => State::Modal,
        _ => return None,
    })
}

pub fn semantic_action_from_ax(action: &str) -> Option<SemanticAction> {
    Some(match action {
        "AXPress" => SemanticAction::Activate,
        "AXIncrement" => SemanticAction::Increment,
        "AXDecrement" => SemanticAction::Decrement,
        "AXConfirm" => SemanticAction::Confirm,
        "AXCancel" => SemanticAction::Cancel,
        "AXRaise" => SemanticAction::RaiseWindow,
        "AXShowMenu" => SemanticAction::ShowMenu,
        _ => return None,
    })
}

pub fn relation_from_ax_attribute(attribute: &str) -> Option<RelationKind> {
    Some(match attribute {
        "AXTitleUIElement" | "AXLabelUIElements" => RelationKind::LabelledBy,
        "AXServesAsTitleForUIElements" => RelationKind::LabelFor,
        "AXLinkedUIElements" => RelationKind::Controls,
        "AXDescription" => RelationKind::DescribedBy,
        _ => return None,
    })
}
