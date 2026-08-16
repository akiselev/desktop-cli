use super::graph::AccessibilityGraph;
use super::model::*;
use super::policy::RedactionPolicy;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationBudget {
    pub max_nodes: usize,
    pub max_text_chars: usize,
    pub max_collection_items: usize,
    pub max_depth: usize,
    pub max_native_property_bytes: usize,
    pub max_millis: u64,
}
impl Default for ObservationBudget {
    fn default() -> Self { Self { max_nodes: 300, max_text_chars: 24_000, max_collection_items: 40, max_depth: 20, max_native_property_bytes: 16_384, max_millis: 2_000 } }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CollectionSummary {
    pub count: usize,
    pub selected_count: usize,
    pub focused_item: Option<ElementRef>,
    pub selected_items: Vec<ElementRef>,
    pub sample: Vec<String>,
    pub query: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedElement {
    pub reference: ElementRef,
    pub role: Role,
    pub name: Option<String>,
    pub value: Option<String>,
    pub states: Vec<State>,
    pub capabilities: Vec<CapabilityKind>,
    pub alias: Option<String>,
    pub importance: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub exposed: BTreeMap<String, String>,
    pub children: Vec<ObservedElement>,
    pub collapsed_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<CollectionSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub schema_version: u32,
    pub session: SessionId,
    pub revision: u64,
    pub view: Option<String>,
    pub roots: Vec<ObservedElement>,
    pub stats: ObservationStats,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ObservationStats {
    pub source_nodes: usize,
    pub exposed_nodes: usize,
    pub pruned_nodes: usize,
    pub collapsed_nodes: usize,
    pub emitted_text_chars: usize,
    pub emitted_property_bytes: usize,
    pub elapsed_micros: u64,
    pub truncated: bool,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationDiff {
    pub schema_version: u32,
    pub from_revision: u64,
    pub to_revision: u64,
    pub appeared: Vec<ElementRef>,
    pub disappeared: Vec<ElementRef>,
    pub changed: Vec<ElementChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementChange { pub reference: ElementRef, pub fields: Vec<String> }

pub struct BudgetTracker {
    pub budget: ObservationBudget,
    pub stats: ObservationStats,
    start: Instant,
    nodes_left: usize,
    text_left: usize,
    property_bytes_left: usize,
}

impl BudgetTracker {
    pub fn new(source_nodes: usize, budget: ObservationBudget) -> Self {
        Self { budget, stats: ObservationStats { source_nodes, ..Default::default() }, start: Instant::now(), nodes_left: budget.max_nodes, text_left: budget.max_text_chars, property_bytes_left: budget.max_native_property_bytes }
    }
    pub fn enter_node(&mut self, depth: usize) -> bool {
        if depth > self.budget.max_depth { self.stop("max-depth"); return false; }
        if self.nodes_left == 0 { self.stop("max-nodes"); return false; }
        if self.start.elapsed() > Duration::from_millis(self.budget.max_millis) { self.stop("deadline"); return false; }
        self.nodes_left -= 1; self.stats.exposed_nodes += 1; true
    }
    pub fn text(&mut self, input: Option<String>) -> Option<String> {
        let Some(input) = input else { return None; };
        if self.text_left == 0 { self.stop("max-text"); return Some(String::new()); }
        let out = truncate_chars(&input, self.text_left);
        let used = out.chars().count(); self.text_left = self.text_left.saturating_sub(used); self.stats.emitted_text_chars += used;
        if used < input.chars().count() { self.stop("max-text"); }
        Some(out)
    }
    pub fn property(&mut self, input: String) -> Option<String> {
        if self.property_bytes_left == 0 { self.stop("max-native-property-bytes"); return None; }
        let max = self.property_bytes_left;
        let out = truncate_utf8_bytes(&input, max);
        let used = out.len(); self.property_bytes_left = self.property_bytes_left.saturating_sub(used); self.stats.emitted_property_bytes += used;
        if used < input.len() { self.stop("max-native-property-bytes"); }
        Some(out)
    }
    pub fn stop(&mut self, reason: &str) { self.stats.truncated = true; if self.stats.stop_reason.is_none() { self.stats.stop_reason = Some(reason.into()); } }
    pub fn finish(mut self) -> ObservationStats { self.stats.elapsed_micros = self.start.elapsed().as_micros().min(u64::MAX as u128) as u64; self.stats }
}

impl Observation {
    pub fn diff(&self, newer: &Observation) -> ObservationDiff {
        let old = flatten(&self.roots); let new = flatten(&newer.roots);
        let old_keys: BTreeSet<_> = old.keys().cloned().collect(); let new_keys: BTreeSet<_> = new.keys().cloned().collect();
        let appeared = new_keys.difference(&old_keys).filter_map(|k| new.get(k).map(|e| e.reference.clone())).collect();
        let disappeared = old_keys.difference(&new_keys).filter_map(|k| old.get(k).map(|e| e.reference.clone())).collect();
        let mut changed = Vec::new();
        for key in old_keys.intersection(&new_keys) {
            let a = old[key]; let b = new[key]; let mut fields = Vec::new();
            if a.name != b.name { fields.push("name".into()); } if a.value != b.value { fields.push("value".into()); }
            if a.states != b.states { fields.push("states".into()); } if a.capabilities != b.capabilities { fields.push("capabilities".into()); }
            if a.alias != b.alias { fields.push("alias".into()); } if a.exposed != b.exposed { fields.push("exposed".into()); }
            if a.collection != b.collection { fields.push("collection".into()); }
            if !fields.is_empty() { changed.push(ElementChange { reference: b.reference.clone(), fields }); }
        }
        ObservationDiff { schema_version: 2, from_revision: self.revision, to_revision: newer.revision, appeared, disappeared, changed }
    }
}

pub fn observe_raw(graph: &AccessibilityGraph, budget: ObservationBudget) -> Observation {
    observe_raw_with_redaction(graph, budget, &RedactionPolicy::default())
}

pub fn observe_raw_with_redaction(graph: &AccessibilityGraph, budget: ObservationBudget, redaction: &RedactionPolicy) -> Observation {
    let mut tracker = BudgetTracker::new(graph.len(), budget); let mut roots = Vec::new();
    for root in graph.roots() { if let Some(observed) = observe_node(graph, *root, 0, &mut tracker, redaction) { roots.push(observed); } }
    let stats = tracker.finish();
    Observation { schema_version: 2, session: graph.session(), revision: graph.revision(), view: None, roots, stats }
}

fn observe_node(graph: &AccessibilityGraph, node: NodeId, depth: usize, tracker: &mut BudgetTracker, redaction: &RedactionPolicy) -> Option<ObservedElement> {
    if !tracker.enter_node(depth) { return None; }
    let element = graph.get(node)?;
    let mut states = element.states.iter().filter_map(|(s,v)| (v == StateValue::True).then_some(s)).collect::<Vec<_>>(); states.sort();
    let mut capabilities = element.capabilities.iter().copied().collect::<Vec<_>>(); capabilities.sort();
    let alias = graph.aliases().iter().find_map(|(a,n)| (*n == node).then_some(a.clone()));
    let name = tracker.text(element.name.clone());
    let raw_value = element.value.as_ref().map(PropertyValue::display_text);
    let value = tracker.text(redaction.redact_value(element, raw_value));
    let mut children = Vec::new(); let child_count = element.children.len();
    for child in element.children.iter().take(tracker.budget.max_collection_items) { if let Some(v) = observe_node(graph, *child, depth + 1, tracker, redaction) { children.push(v); } }
    let collapsed_count = (child_count > children.len()).then_some(child_count - children.len());
    Some(ObservedElement { reference: graph.element_ref_for(node), role: element.role, name, value, states, capabilities, alias, importance: "normal".into(), exposed: BTreeMap::new(), children, collapsed_count, collection: None })
}

pub fn summarize_collection(graph: &AccessibilityGraph, node: NodeId, max_sample: usize) -> CollectionSummary {
    let Some(element) = graph.get(node) else { return CollectionSummary::default(); };
    let mut selected_items = Vec::new(); let mut focused_item = None; let mut sample = Vec::new();
    for child in &element.children {
        let Some(item) = graph.get(*child) else { continue; };
        if item.states.is_true(State::Selected) { selected_items.push(graph.element_ref_for(*child)); }
        if focused_item.is_none() && item.states.is_true(State::Focused) { focused_item = Some(graph.element_ref_for(*child)); }
        if sample.len() < max_sample { if let Some(name) = &item.name { sample.push(name.clone()); } }
    }
    CollectionSummary { count: element.children.len(), selected_count: selected_items.len(), focused_item, selected_items: selected_items.into_iter().take(max_sample).collect(), sample, query: Some(format!("{} > *", graph.element_ref_for(node).opaque)) }
}

fn flatten<'a>(roots: &'a [ObservedElement]) -> BTreeMap<String, &'a ObservedElement> {
    fn walk<'a>(node: &'a ObservedElement, out: &mut BTreeMap<String, &'a ObservedElement>) { out.insert(node.reference.opaque.clone(), node); for child in &node.children { walk(child, out); } }
    let mut out = BTreeMap::new(); for root in roots { walk(root, &mut out); } out
}
fn truncate_chars(input: &str, max: usize) -> String { input.chars().take(max).collect() }
fn truncate_utf8_bytes(input: &str, max: usize) -> String { if input.len() <= max { return input.to_string(); } let mut end = max.min(input.len()); while end > 0 && !input.is_char_boundary(end) { end -= 1; } input[..end].to_string() }
