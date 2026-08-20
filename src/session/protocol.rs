use crate::packs::{parse_manifest, parse_stylesheet, ApplicationPack};
use crate::semantic::{ActionValue, ObservationBudget, SemanticAction};
use super::DesktopSession;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "kebab-case")]
pub enum SessionRequest {
    Hello,
    Refresh,
    Query { selector: String },
    ExplainQuery { selector: String },
    Observe { budget: Option<ObservationBudget> },
    Action { target: String, action: String, value: Option<Value>, allow_input_fallback: Option<bool> },
    SetPack { manifest: String, stylesheet: String },
    ClearPack,
    PollEvents { max_events: Option<usize> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

impl SessionResponse {
    fn ok<T: Serialize>(value: T) -> Self { Self { ok: true, result: serde_json::to_value(value).ok(), error: None } }
    fn err(error: impl Into<String>) -> Self { Self { ok: false, result: None, error: Some(error.into()) } }
}

pub fn handle_request(session: &mut DesktopSession, request: SessionRequest) -> SessionResponse {
    let result: Result<Value, String> = (|| match request {
        SessionRequest::Hello => serde_json::to_value(serde_json::json!({"protocol":1,"session":session.id,"backend":session.backend_kind()})).map_err(|e|e.to_string()),
        SessionRequest::Refresh => session.refresh().and_then(|r|serde_json::to_value(serde_json::json!({"revision":r})).map_err(|e|e.to_string())),
        SessionRequest::Query { selector } => session.query(&selector).and_then(|v|serde_json::to_value(v).map_err(|e|e.to_string())),
        SessionRequest::ExplainQuery { selector } => session.explain_query(&selector).and_then(|v|serde_json::to_value(v).map_err(|e|e.to_string())),
        SessionRequest::Observe { budget } => session.observe(budget.unwrap_or_default()).and_then(|v|serde_json::to_value(v).map_err(|e|e.to_string())),
        SessionRequest::Action { target, action, value, allow_input_fallback } => {
            let action = SemanticAction::parse(&action).ok_or_else(||format!("unknown semantic action '{}'",action))?;
            let value = match value { None|Some(Value::Null)=>ActionValue::None, Some(Value::String(s))=>ActionValue::Text(s), Some(Value::Bool(v))=>ActionValue::Bool(v), Some(Value::Number(n))=>ActionValue::Number(n.as_f64().ok_or_else(||"invalid numeric value".to_string())?), Some(v)=>ActionValue::Text(v.to_string()) };
            session.perform(&target, action, value, allow_input_fallback.unwrap_or(false)).and_then(|v|serde_json::to_value(v).map_err(|e|e.to_string()))
        }
        SessionRequest::SetPack { manifest, stylesheet } => {
            let manifest = parse_manifest(&manifest).map_err(|d|format!("manifest: {:?}",d))?;
            let stylesheet = parse_stylesheet(&stylesheet).map_err(|d|format!("stylesheet: {:?}",d))?;
            session.set_pack(Some(ApplicationPack { manifest, stylesheet })); Ok(Value::Bool(true))
        }
        SessionRequest::ClearPack => { session.set_pack(None); Ok(Value::Bool(true)) }
        SessionRequest::PollEvents { max_events } => session.apply_pending_events(max_events.unwrap_or(128)).and_then(|n|serde_json::to_value(serde_json::json!({"events":n})).map_err(|e|e.to_string())),
    })();
    match result { Ok(v)=>SessionResponse::ok(v), Err(e)=>SessionResponse::err(e) }
}

pub fn run_jsonl_server<R: BufRead, W: Write>(session: &mut DesktopSession, input: R, mut output: W) -> Result<(), String> {
    for line in input.lines() {
        let line = line.map_err(|e|e.to_string())?; if line.trim().is_empty(){continue;}
        let response = match serde_json::from_str::<SessionRequest>(&line) { Ok(req)=>handle_request(session,req), Err(e)=>SessionResponse::err(format!("invalid request: {}",e)) };
        serde_json::to_writer(&mut output,&response).map_err(|e|e.to_string())?; output.write_all(b"\n").map_err(|e|e.to_string())?; output.flush().map_err(|e|e.to_string())?;
    }
    Ok(())
}
