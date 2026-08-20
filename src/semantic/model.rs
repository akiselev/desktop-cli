use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    WindowsUia,
    LinuxAtSpi,
    MacAx,
    Mock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SessionId(pub u64);

impl SessionId {
    pub fn new() -> Self {
        Self(NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

pub type NodeId = u64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ElementRef {
    pub session: SessionId,
    pub generation: u64,
    pub opaque: String,
}

impl fmt::Display for ElementRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.opaque)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "kebab-case")]
pub enum NativeIdentity {
    Uia {
        runtime_id: Vec<i32>,
        process_id: Option<u32>,
    },
    AtSpi {
        bus_name: String,
        object_path: String,
    },
    Ax {
        registry_token: u64,
        pid: i32,
    },
    Synthetic {
        key: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementIdentity {
    pub native: NativeIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stable_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }

    pub fn contains(&self, other: &Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }

    pub fn overlaps_x(&self, other: &Rect) -> bool {
        self.x < other.x + other.width && other.x < self.x + self.width
    }

    pub fn overlaps_y(&self, other: &Rect) -> bool {
        self.y < other.y + other.height && other.y < self.y + self.height
    }

    pub fn distance_to(&self, other: &Rect) -> f64 {
        let a = self.center();
        let b = other.center();
        ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum PropertyValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    String(String),
    Point(Point),
    Size(Size),
    Rect(Rect),
    Range(TextRange),
    Ref(ElementRef),
    Refs(Vec<ElementRef>),
    Array(Vec<PropertyValue>),
    Map(BTreeMap<String, PropertyValue>),
    Opaque {
        type_name: String,
        debug: Option<String>,
    },
}

impl PropertyValue {
    pub fn display_text(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(v) => v.to_string(),
            Self::I64(v) => v.to_string(),
            Self::U64(v) => v.to_string(),
            Self::F64(v) => v.to_string(),
            Self::String(v) => v.clone(),
            Self::Point(v) => format!("{},{}", v.x, v.y),
            Self::Size(v) => format!("{}x{}", v.width, v.height),
            Self::Rect(v) => format!("{},{},{},{}", v.x, v.y, v.width, v.height),
            Self::Range(v) => format!("{}..{}", v.start, v.end),
            Self::Ref(v) => v.opaque.clone(),
            Self::Refs(v) => v.iter().map(|r| r.opaque.as_str()).collect::<Vec<_>>().join(","),
            Self::Array(v) => v.iter().map(Self::display_text).collect::<Vec<_>>().join(","),
            Self::Map(_) => "[map]".into(),
            Self::Opaque { type_name, debug } => debug.clone().unwrap_or_else(|| type_name.clone()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Desktop,
    Application,
    Window,
    Dialog,
    Alert,
    Button,
    Checkbox,
    RadioButton,
    ToggleButton,
    TextInput,
    SearchInput,
    PasswordInput,
    Text,
    StaticText,
    Link,
    Image,
    MenuBar,
    Menu,
    MenuItem,
    Toolbar,
    TabList,
    Tab,
    List,
    ListItem,
    Tree,
    TreeItem,
    Table,
    Row,
    Column,
    Cell,
    Header,
    ComboBox,
    Slider,
    SpinButton,
    ScrollBar,
    Document,
    Canvas,
    Group,
    Panel,
    Splitter,
    Progress,
    StatusBar,
    Tooltip,
    Unknown,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Application => "application",
            Self::Window => "window",
            Self::Dialog => "dialog",
            Self::Alert => "alert",
            Self::Button => "button",
            Self::Checkbox => "checkbox",
            Self::RadioButton => "radio",
            Self::ToggleButton => "toggle-button",
            Self::TextInput => "input",
            Self::SearchInput => "search",
            Self::PasswordInput => "password",
            Self::Text => "text",
            Self::StaticText => "static-text",
            Self::Link => "link",
            Self::Image => "image",
            Self::MenuBar => "menubar",
            Self::Menu => "menu",
            Self::MenuItem => "menuitem",
            Self::Toolbar => "toolbar",
            Self::TabList => "tablist",
            Self::Tab => "tab",
            Self::List => "list",
            Self::ListItem => "listitem",
            Self::Tree => "tree",
            Self::TreeItem => "treeitem",
            Self::Table => "table",
            Self::Row => "row",
            Self::Column => "column",
            Self::Cell => "cell",
            Self::Header => "header",
            Self::ComboBox => "combobox",
            Self::Slider => "slider",
            Self::SpinButton => "spinbutton",
            Self::ScrollBar => "scrollbar",
            Self::Document => "document",
            Self::Canvas => "canvas",
            Self::Group => "group",
            Self::Panel => "panel",
            Self::Splitter => "splitter",
            Self::Progress => "progress",
            Self::StatusBar => "statusbar",
            Self::Tooltip => "tooltip",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(input: &str) -> Self {
        match input.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "desktop" => Self::Desktop,
            "application" | "app" => Self::Application,
            "window" => Self::Window,
            "dialog" => Self::Dialog,
            "alert" => Self::Alert,
            "button" | "btn" => Self::Button,
            "checkbox" | "check" => Self::Checkbox,
            "radio" | "radiobutton" | "radio-button" => Self::RadioButton,
            "toggle" | "toggle-button" => Self::ToggleButton,
            "input" | "edit" | "textbox" | "text-input" => Self::TextInput,
            "search" | "search-input" => Self::SearchInput,
            "password" | "password-input" => Self::PasswordInput,
            "text" => Self::Text,
            "static-text" | "label" => Self::StaticText,
            "link" | "hyperlink" => Self::Link,
            "image" => Self::Image,
            "menubar" | "menu-bar" => Self::MenuBar,
            "menu" => Self::Menu,
            "menuitem" | "menu-item" => Self::MenuItem,
            "toolbar" => Self::Toolbar,
            "tablist" | "tab-list" => Self::TabList,
            "tab" | "tabitem" => Self::Tab,
            "list" => Self::List,
            "listitem" | "list-item" => Self::ListItem,
            "tree" => Self::Tree,
            "treeitem" | "tree-item" => Self::TreeItem,
            "table" | "grid" | "datagrid" => Self::Table,
            "row" => Self::Row,
            "column" => Self::Column,
            "cell" | "dataitem" => Self::Cell,
            "header" => Self::Header,
            "combobox" | "combo-box" | "dropdown" => Self::ComboBox,
            "slider" => Self::Slider,
            "spinbutton" | "spin-button" | "spinner" => Self::SpinButton,
            "scrollbar" | "scroll-bar" => Self::ScrollBar,
            "document" => Self::Document,
            "canvas" => Self::Canvas,
            "group" => Self::Group,
            "panel" | "pane" => Self::Panel,
            "splitter" => Self::Splitter,
            "progress" | "progressbar" => Self::Progress,
            "statusbar" | "status-bar" => Self::StatusBar,
            "tooltip" | "tool-tip" => Self::Tooltip,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeRole {
    pub backend: BackendKind,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subrole: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numeric_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    Active,
    Busy,
    Enabled,
    Sensitive,
    Focusable,
    Focused,
    Editable,
    ReadOnly,
    Selectable,
    Selected,
    MultiSelectable,
    Checkable,
    Checked,
    Indeterminate,
    Expandable,
    Expanded,
    Modal,
    Visible,
    Showing,
    Offscreen,
    Defunct,
    MultiLine,
    Required,
    Invalid,
    Pressed,
    Visited,
    Protected,
}

impl State {
    pub fn parse(input: &str) -> Option<Self> {
        Some(match input.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "active" => Self::Active,
            "busy" => Self::Busy,
            "enabled" => Self::Enabled,
            "sensitive" => Self::Sensitive,
            "focusable" => Self::Focusable,
            "focused" | "focus" => Self::Focused,
            "editable" => Self::Editable,
            "read-only" | "readonly" => Self::ReadOnly,
            "selectable" => Self::Selectable,
            "selected" => Self::Selected,
            "multi-selectable" | "multiselectable" => Self::MultiSelectable,
            "checkable" => Self::Checkable,
            "checked" => Self::Checked,
            "indeterminate" | "mixed" => Self::Indeterminate,
            "expandable" => Self::Expandable,
            "expanded" => Self::Expanded,
            "modal" => Self::Modal,
            "visible" => Self::Visible,
            "showing" => Self::Showing,
            "offscreen" | "off-screen" => Self::Offscreen,
            "defunct" => Self::Defunct,
            "multi-line" | "multiline" => Self::MultiLine,
            "required" => Self::Required,
            "invalid" => Self::Invalid,
            "pressed" => Self::Pressed,
            "visited" => Self::Visited,
            "protected" => Self::Protected,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StateValue {
    True,
    False,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StateSet {
    values: BTreeMap<State, StateValue>,
}

impl StateSet {
    pub fn get(&self, state: State) -> StateValue {
        self.values.get(&state).copied().unwrap_or(StateValue::Unknown)
    }

    pub fn set(&mut self, state: State, value: StateValue) {
        self.values.insert(state, value);
    }

    pub fn set_bool(&mut self, state: State, value: bool) {
        self.set(
            state,
            if value {
                StateValue::True
            } else {
                StateValue::False
            },
        );
    }

    pub fn is_true(&self, state: State) -> bool {
        self.get(state) == StateValue::True
    }

    pub fn iter(&self) -> impl Iterator<Item = (State, StateValue)> + '_ {
        self.values.iter().map(|(k, v)| (*k, *v))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelationKind {
    LabelFor,
    LabelledBy,
    Controls,
    ControlledBy,
    MemberOf,
    TooltipFor,
    FlowsTo,
    FlowsFrom,
    SubwindowOf,
    ParentWindowOf,
    Embeds,
    EmbeddedBy,
    DescribedBy,
    DescriptionFor,
    Details,
    DetailsFor,
    ErrorFor,
    ErrorMessage,
    PopupFor,
    Native(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    pub kind: RelationKind,
    pub targets: Vec<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoordinateSpace {
    ScreenLogical,
    ScreenPhysical,
    Window,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    Visible,
    Hidden,
    Offscreen,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    pub bounds: Rect,
    pub coordinate_space: CoordinateSpace,
    pub visibility: Visibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_order: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityKind {
    Action,
    Value,
    RangeValue,
    Toggle,
    ExpandCollapse,
    Text,
    EditableText,
    Selection,
    SelectionItem,
    Table,
    TableCell,
    Document,
    Component,
    HitTest,
    Scroll,
    ScrollItem,
    Window,
    Transform,
    Drag,
    DropTarget,
    VirtualizedItem,
    ItemContainer,
    Hypertext,
    Image,
}

impl CapabilityKind {
    pub fn parse(input: &str) -> Option<Self> {
        Some(match input.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "action" => Self::Action,
            "value" => Self::Value,
            "range-value" => Self::RangeValue,
            "toggle" => Self::Toggle,
            "expand-collapse" => Self::ExpandCollapse,
            "text" => Self::Text,
            "editable-text" => Self::EditableText,
            "selection" => Self::Selection,
            "selection-item" => Self::SelectionItem,
            "table" => Self::Table,
            "table-cell" => Self::TableCell,
            "document" => Self::Document,
            "component" => Self::Component,
            "hit-test" => Self::HitTest,
            "scroll" => Self::Scroll,
            "scroll-item" => Self::ScrollItem,
            "window" => Self::Window,
            "transform" => Self::Transform,
            "drag" => Self::Drag,
            "drop-target" => Self::DropTarget,
            "virtualized-item" => Self::VirtualizedItem,
            "item-container" => Self::ItemContainer,
            "hypertext" => Self::Hypertext,
            "image" => Self::Image,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticAction {
    Activate,
    Focus,
    SetValue,
    Toggle,
    Select,
    Expand,
    Collapse,
    Increment,
    Decrement,
    ShowMenu,
    ScrollIntoView,
    Confirm,
    Cancel,
    RaiseWindow,
    SetText,
    ReplaceText,
}

impl SemanticAction {
    pub fn parse(input: &str) -> Option<Self> {
        Some(match input.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "activate" | "invoke" | "click" | "press" => Self::Activate,
            "focus" => Self::Focus,
            "set-value" | "set" => Self::SetValue,
            "toggle" => Self::Toggle,
            "select" => Self::Select,
            "expand" => Self::Expand,
            "collapse" => Self::Collapse,
            "increment" => Self::Increment,
            "decrement" => Self::Decrement,
            "show-menu" => Self::ShowMenu,
            "scroll-into-view" => Self::ScrollIntoView,
            "confirm" => Self::Confirm,
            "cancel" => Self::Cancel,
            "raise-window" => Self::RaiseWindow,
            "set-text" => Self::SetText,
            "replace-text" => Self::ReplaceText,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeAction {
    pub backend: BackendKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElementAction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic: Option<SemanticAction>,
    pub native: NativeAction,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_binding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NativeMetadata {
    pub attributes: BTreeMap<String, PropertyValue>,
    pub interfaces: BTreeSet<String>,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementSnapshot {
    pub node: NodeId,
    pub identity: ElementIdentity,
    pub role: Role,
    pub native_role: NativeRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<PropertyValue>,
    pub states: StateSet,
    pub capabilities: BTreeSet<CapabilityKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ElementAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Geometry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<NodeId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Relation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, PropertyValue>,
    pub native: NativeMetadata,
}

impl ElementSnapshot {
    pub fn property_text(&self, name: &str) -> Option<String> {
        match name {
            "name" => self.name.clone(),
            "description" => self.description.clone(),
            "value" => self.value.as_ref().map(PropertyValue::display_text),
            "role" => Some(self.role.as_str().into()),
            "stable-id" | "id" => self.identity.stable_id.clone(),
            other => self
                .properties
                .get(other)
                .map(PropertyValue::display_text)
                .or_else(|| {
                    self.native
                        .attributes
                        .get(other.trim_start_matches("native."))
                        .map(PropertyValue::display_text)
                }),
        }
    }
}
