//! UIA pattern execution (Invoke, Value, Toggle, Selection, etc.)

use crate::rpc::types::PatternResult;
use uiautomation::patterns::{
    UIExpandCollapsePattern, UIInvokePattern, UIScrollItemPattern, UISelectionItemPattern,
    UITextPattern, UITogglePattern, UIValuePattern,
};
use uiautomation::types::ExpandCollapseState;
use uiautomation::UIElement;

/// Supported pattern operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternOp {
    /// Invoke pattern - click buttons, menu items
    Invoke,
    /// Get value from Value pattern
    GetValue,
    /// Set value in Value pattern
    SetValue,
    /// Toggle (checkbox, toggle button)
    Toggle,
    /// Get toggle state
    GetToggleState,
    /// Select item via SelectionItem pattern
    Select,
    /// Deselect item
    Deselect,
    /// Check if selected
    IsSelected,
    /// Expand via ExpandCollapse pattern
    Expand,
    /// Collapse via ExpandCollapse pattern
    Collapse,
    /// Get expand/collapse state
    GetExpandState,
    /// Get text via Text pattern
    GetText,
    /// Scroll into view
    ScrollIntoView,
    /// Focus the element
    Focus,
}

impl PatternOp {
    /// Parse a pattern operation from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace(['-', '_'], "").as_str() {
            "invoke" | "click" => Some(Self::Invoke),
            "getvalue" | "readvalue" | "value" => Some(Self::GetValue),
            "setvalue" | "writevalue" | "type" => Some(Self::SetValue),
            "toggle" => Some(Self::Toggle),
            "gettogglestate" | "togglestate" => Some(Self::GetToggleState),
            "select" => Some(Self::Select),
            "deselect" | "unselect" => Some(Self::Deselect),
            "isselected" | "selected" => Some(Self::IsSelected),
            "expand" => Some(Self::Expand),
            "collapse" => Some(Self::Collapse),
            "getexpandstate" | "expandstate" => Some(Self::GetExpandState),
            "gettext" | "text" | "readtext" => Some(Self::GetText),
            "scrollintoview" | "scroll" => Some(Self::ScrollIntoView),
            "focus" | "setfocus" => Some(Self::Focus),
            _ => None,
        }
    }
}

/// Execute a pattern operation on an element
pub fn execute_pattern(elem: &UIElement, op: PatternOp, value: Option<&str>) -> PatternResult {
    match op {
        PatternOp::Invoke => invoke(elem),
        PatternOp::GetValue => get_value(elem),
        PatternOp::SetValue => {
            let val = value.unwrap_or("");
            set_value(elem, val)
        }
        PatternOp::Toggle => toggle(elem),
        PatternOp::GetToggleState => get_toggle_state(elem),
        PatternOp::Select => select(elem),
        PatternOp::Deselect => deselect(elem),
        PatternOp::IsSelected => is_selected(elem),
        PatternOp::Expand => expand(elem),
        PatternOp::Collapse => collapse(elem),
        PatternOp::GetExpandState => get_expand_state(elem),
        PatternOp::GetText => get_text(elem),
        PatternOp::ScrollIntoView => scroll_into_view(elem),
        PatternOp::Focus => focus(elem),
    }
}

fn invoke(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UIInvokePattern>() {
        Ok(pattern) => match pattern.invoke() {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(format!("Invoke failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("Invoke pattern not supported: {}", e)),
    }
}

fn get_value(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UIValuePattern>() {
        Ok(pattern) => match pattern.get_value() {
            Ok(val) => PatternResult::ok_with_value(val),
            Err(e) => PatternResult::err(format!("GetValue failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("Value pattern not supported: {}", e)),
    }
}

fn set_value(elem: &UIElement, value: &str) -> PatternResult {
    match elem.get_pattern::<UIValuePattern>() {
        Ok(pattern) => match pattern.set_value(value) {
            Ok(()) => PatternResult::ok_with_value(value.to_string()),
            Err(e) => PatternResult::err(format!("SetValue failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("Value pattern not supported: {}", e)),
    }
}

fn toggle(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UITogglePattern>() {
        Ok(pattern) => match pattern.toggle() {
            Ok(()) => {
                // Get the new state
                match pattern.get_toggle_state() {
                    Ok(state) => PatternResult::ok_with_value(format!("{:?}", state)),
                    Err(_) => PatternResult::ok(),
                }
            }
            Err(e) => PatternResult::err(format!("Toggle failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("Toggle pattern not supported: {}", e)),
    }
}

fn get_toggle_state(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UITogglePattern>() {
        Ok(pattern) => match pattern.get_toggle_state() {
            Ok(state) => PatternResult::ok_with_value(format!("{:?}", state)),
            Err(e) => PatternResult::err(format!("GetToggleState failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("Toggle pattern not supported: {}", e)),
    }
}

fn select(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UISelectionItemPattern>() {
        Ok(pattern) => match pattern.select() {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(format!("Select failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("SelectionItem pattern not supported: {}", e)),
    }
}

fn deselect(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UISelectionItemPattern>() {
        Ok(pattern) => match pattern.remove_from_selection() {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(format!("Deselect failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("SelectionItem pattern not supported: {}", e)),
    }
}

fn is_selected(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UISelectionItemPattern>() {
        Ok(pattern) => match pattern.is_selected() {
            Ok(selected) => PatternResult::ok_with_value(selected.to_string()),
            Err(e) => PatternResult::err(format!("IsSelected failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("SelectionItem pattern not supported: {}", e)),
    }
}

fn expand(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UIExpandCollapsePattern>() {
        Ok(pattern) => match pattern.expand() {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(format!("Expand failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("ExpandCollapse pattern not supported: {}", e)),
    }
}

fn collapse(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UIExpandCollapsePattern>() {
        Ok(pattern) => match pattern.collapse() {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(format!("Collapse failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("ExpandCollapse pattern not supported: {}", e)),
    }
}

fn get_expand_state(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UIExpandCollapsePattern>() {
        Ok(pattern) => match pattern.get_state() {
            Ok(state) => {
                let state_str = match state {
                    ExpandCollapseState::Collapsed => "Collapsed",
                    ExpandCollapseState::Expanded => "Expanded",
                    ExpandCollapseState::PartiallyExpanded => "PartiallyExpanded",
                    ExpandCollapseState::LeafNode => "LeafNode",
                };
                PatternResult::ok_with_value(state_str.to_string())
            }
            Err(e) => PatternResult::err(format!("GetExpandState failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("ExpandCollapse pattern not supported: {}", e)),
    }
}

fn get_text(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UITextPattern>() {
        Ok(pattern) => match pattern.get_document_range() {
            Ok(range) => match range.get_text(-1) {
                Ok(text) => PatternResult::ok_with_value(text),
                Err(e) => PatternResult::err(format!("GetText failed: {}", e)),
            },
            Err(e) => PatternResult::err(format!("GetDocumentRange failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("Text pattern not supported: {}", e)),
    }
}

fn scroll_into_view(elem: &UIElement) -> PatternResult {
    match elem.get_pattern::<UIScrollItemPattern>() {
        Ok(pattern) => match pattern.scroll_into_view() {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(format!("ScrollIntoView failed: {}", e)),
        },
        Err(e) => PatternResult::err(format!("ScrollItem pattern not supported: {}", e)),
    }
}

fn focus(elem: &UIElement) -> PatternResult {
    match elem.set_focus() {
        Ok(()) => PatternResult::ok(),
        Err(e) => PatternResult::err(format!("Focus failed: {}", e)),
    }
}
