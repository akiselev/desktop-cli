use super::model::*;
use super::selector::Selector;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InspectMask {
    pub identity: bool,
    pub properties: bool,
    pub states: bool,
    pub capabilities: bool,
    pub actions: bool,
    pub relations: bool,
    pub geometry: bool,
}

impl InspectMask {
    pub const BASIC: Self = Self { identity: true, properties: true, states: true, capabilities: true, actions: false, relations: false, geometry: true };
    pub const FULL: Self = Self { identity: true, properties: true, states: true, capabilities: true, actions: true, relations: true, geometry: true };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildRange { pub offset: usize, pub limit: usize }
impl Default for ChildRange { fn default() -> Self { Self { offset: 0, limit: 256 } } }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendRoot {
    pub native: NativeIdentity,
    pub name: Option<String>,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionValue { None, Text(String), Bool(bool), Number(f64), Range(TextRange) }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionRequest {
    pub action: SemanticAction,
    pub value: ActionValue,
    pub native_hint: Option<NativeAction>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackendActionResult {
    pub success: bool,
    pub native_action: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextQuery {
    All,
    Range(TextRange),
    Line(usize),
    Character(usize),
    Bounds(TextRange),
    Attributes(TextRange),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TextResult {
    Text(String),
    Bounds(Vec<Rect>),
    Attributes(Vec<(String, PropertyValue)>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventKind { Created, Destroyed, ChildrenChanged, PropertyChanged, StateChanged, FocusChanged, SelectionChanged, TextChanged, GeometryChanged, WindowCreated, WindowDestroyed }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessibilityEvent {
    Created(NativeIdentity),
    Destroyed(NativeIdentity),
    ChildrenChanged(NativeIdentity),
    PropertyChanged { element: NativeIdentity, property: String },
    StateChanged { element: NativeIdentity, state: State, value: bool },
    FocusChanged { old: Option<NativeIdentity>, new: Option<NativeIdentity> },
    SelectionChanged(NativeIdentity),
    TextChanged { element: NativeIdentity, range: Option<TextRange> },
    GeometryChanged(NativeIdentity),
    WindowCreated(NativeIdentity),
    WindowDestroyed(NativeIdentity),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeQueryPlan {
    pub backend: BackendKind,
    pub description: String,
    pub pushed_predicates: Vec<String>,
    pub residual_selector: String,
    pub exact: bool,
}

pub trait AccessibilityBackend: Send {
    fn kind(&self) -> BackendKind;
    fn roots(&mut self) -> Result<Vec<BackendRoot>, String>;
    fn inspect(&mut self, identity: &NativeIdentity, mask: InspectMask) -> Result<ElementSnapshot, String>;
    fn children(&mut self, identity: &NativeIdentity, range: ChildRange) -> Result<Vec<NativeIdentity>, String>;
    fn perform(&mut self, identity: &NativeIdentity, request: ActionRequest) -> Result<BackendActionResult, String>;

    fn relations(&mut self, identity: &NativeIdentity) -> Result<Vec<Relation>, String> {
        Ok(self.inspect(identity, InspectMask::FULL)?.relations)
    }
    fn query_text(&mut self, _identity: &NativeIdentity, _query: TextQuery) -> Result<TextResult, String> { Err("text capability is not implemented by this backend".into()) }
    fn plan_query(&self, selector: &Selector) -> NativeQueryPlan {
        NativeQueryPlan { backend: self.kind(), description: "local semantic graph scan".into(), pushed_predicates: vec![], residual_selector: selector.source.clone(), exact: false }
    }
    fn subscribe(&mut self, _root: &NativeIdentity, _interests: &BTreeSet<EventKind>) -> Result<Receiver<AccessibilityEvent>, String> { Err("event subscription is not implemented by this backend".into()) }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowInfoV2 {
    pub id: String,
    pub title: String,
    pub executable: String,
    pub pid: u32,
    pub bounds: Option<Rect>,
    pub native_class: Option<String>,
    pub bundle_id: Option<String>,
}

pub trait WindowBackend: Send {
    fn list_windows(&mut self) -> Result<Vec<WindowInfoV2>, String>;
    fn focus_window(&mut self, id: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton { Left, Right, Middle }

pub trait InputBackend: Send {
    fn click(&mut self, point: Point, button: MouseButton) -> Result<(), String>;
    fn double_click(&mut self, point: Point, button: MouseButton) -> Result<(), String>;
    fn type_text(&mut self, text: &str) -> Result<(), String>;
    fn keys(&mut self, keys: &str) -> Result<(), String>;
    fn scroll(&mut self, delta_x: f64, delta_y: f64, at: Option<Point>) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capture { pub width: u32, pub height: u32, pub format: String, pub bytes: Vec<u8> }

pub trait CaptureBackend: Send {
    fn capture_window(&mut self, window_id: &str) -> Result<Capture, String>;
    fn capture_region(&mut self, region: Rect) -> Result<Capture, String>;
}
