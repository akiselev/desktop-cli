use super::backend::{AccessibilityBackend, ActionRequest, ActionValue, InputBackend, MouseButton};
use super::graph::AccessibilityGraph;
use super::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticActionRequest {
    pub target: ElementRef,
    pub action: SemanticAction,
    pub value: ActionValue,
    pub allow_input_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticActionResult {
    pub success: bool,
    pub target: ElementRef,
    pub action: SemanticAction,
    pub route: String,
    pub message: Option<String>,
}

pub struct ActionRouter<'a> {
    pub graph: &'a AccessibilityGraph,
    pub accessibility: &'a mut (dyn AccessibilityBackend + 'static),
    pub input: Option<&'a mut (dyn InputBackend + 'static)>,
}

impl<'a> ActionRouter<'a> {
    pub fn perform(&mut self, request: SemanticActionRequest) -> Result<SemanticActionResult, String> {
        let node = self.graph.resolve_ref(&request.target)?;
        let element = self
            .graph
            .get(node)
            .ok_or_else(|| "target disappeared".to_string())?;
        let backend_result = self.accessibility.perform(
            &element.identity.native,
            ActionRequest {
                action: request.action,
                value: request.value.clone(),
                native_hint: element
                    .actions
                    .iter()
                    .find(|a| a.semantic == Some(request.action))
                    .map(|a| a.native.clone()),
            },
        );
        match backend_result {
            Ok(result) if result.success => Ok(SemanticActionResult {
                success: true,
                target: request.target,
                action: request.action,
                route: "accessibility".into(),
                message: result.message,
            }),
            Ok(result) if !request.allow_input_fallback => Ok(SemanticActionResult {
                success: false,
                target: request.target,
                action: request.action,
                route: "accessibility".into(),
                message: result.message,
            }),
            Err(error) if !request.allow_input_fallback => Err(error),
            backend => {
                let input = self.input.as_deref_mut().ok_or_else(|| match backend {
                    Ok(r) => r.message.unwrap_or_else(|| {
                        "accessibility action failed and no input backend is available".into()
                    }),
                    Err(e) => e,
                })?;
                let geometry = element
                    .geometry
                    .ok_or_else(|| "input fallback requires target geometry".to_string())?;
                match request.action {
                    SemanticAction::Activate
                    | SemanticAction::Focus
                    | SemanticAction::Select
                    | SemanticAction::Toggle
                    | SemanticAction::ShowMenu => {
                        input.click(geometry.bounds.center(), MouseButton::Left)?
                    }
                    SemanticAction::SetValue
                    | SemanticAction::SetText
                    | SemanticAction::ReplaceText => {
                        input.click(geometry.bounds.center(), MouseButton::Left)?;
                        if let ActionValue::Text(text) = request.value {
                            input.type_text(&text)?;
                        } else {
                            return Err("text input fallback requires a text value".into());
                        }
                    }
                    SemanticAction::ScrollIntoView => {
                        input.scroll(0.0, -3.0, Some(geometry.bounds.center()))?
                    }
                    _ => {
                        return Err(format!(
                            "no safe input fallback for {:?}",
                            request.action
                        ))
                    }
                }
                Ok(SemanticActionResult {
                    success: true,
                    target: request.target,
                    action: request.action,
                    route: "input-fallback".into(),
                    message: Some(
                        "native accessibility action unavailable; used guarded input fallback"
                            .into(),
                    ),
                })
            }
        }
    }
}
