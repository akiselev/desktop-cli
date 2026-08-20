use super::model::{ElementSnapshot, Role, SemanticAction, State};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionClass { Activate, Focus, Edit, Toggle, Select, Navigate, Window }

impl ActionClass {
    pub fn for_action(action: SemanticAction) -> Self {
        match action {
            SemanticAction::Activate | SemanticAction::Confirm | SemanticAction::Cancel => Self::Activate,
            SemanticAction::Focus => Self::Focus,
            SemanticAction::SetValue | SemanticAction::SetText | SemanticAction::ReplaceText => Self::Edit,
            SemanticAction::Toggle => Self::Toggle,
            SemanticAction::Select => Self::Select,
            SemanticAction::Expand | SemanticAction::Collapse | SemanticAction::Increment | SemanticAction::Decrement | SemanticAction::ShowMenu | SemanticAction::ScrollIntoView => Self::Navigate,
            SemanticAction::RaiseWindow => Self::Window,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WindowScope { Any, CurrentSession, Titles(Vec<String>) }
impl Default for WindowScope { fn default() -> Self { Self::CurrentSession } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPolicy {
    pub allowed_windows: WindowScope,
    pub allow_native_actions: bool,
    pub allow_keyboard_fallback: bool,
    pub allow_pointer_fallback: bool,
    pub denied_actions: BTreeSet<ActionClass>,
    pub deny_sensitive_edits: bool,
}

impl Default for ActionPolicy {
    fn default() -> Self { Self { allowed_windows: WindowScope::CurrentSession, allow_native_actions: true, allow_keyboard_fallback: false, allow_pointer_fallback: false, denied_actions: BTreeSet::new(), deny_sensitive_edits: true } }
}

impl ActionPolicy {
    pub fn check(&self, element: &ElementSnapshot, action: SemanticAction, pointer_fallback: bool, keyboard_fallback: bool) -> Result<(), String> {
        let class = ActionClass::for_action(action);
        if self.denied_actions.contains(&class) { return Err(format!("action class {:?} is denied by policy", class)); }
        if !self.allow_native_actions && !pointer_fallback && !keyboard_fallback { return Err("native accessibility actions are disabled by policy".into()); }
        if pointer_fallback && !self.allow_pointer_fallback { return Err("pointer fallback is disabled by policy".into()); }
        if keyboard_fallback && !self.allow_keyboard_fallback { return Err("keyboard fallback is disabled by policy".into()); }
        if self.deny_sensitive_edits && matches!(class, ActionClass::Edit) && (element.states.is_true(State::Protected) || element.role == Role::PasswordInput) { return Err("editing sensitive text is denied by policy".into()); }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionPolicy { pub redact_sensitive_values: bool, pub replacement: String }
impl Default for RedactionPolicy { fn default() -> Self { Self { redact_sensitive_values: true, replacement: "[redacted]".into() } } }
impl RedactionPolicy {
    pub fn redact_value(&self, element: &ElementSnapshot, value: Option<String>) -> Option<String> {
        if self.redact_sensitive_values && value.is_some() && (element.states.is_true(State::Protected) || element.role == Role::PasswordInput) { Some(self.replacement.clone()) } else { value }
    }
}
