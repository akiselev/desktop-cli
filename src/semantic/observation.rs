use super::graph::AccessibilityGraph;
use super::model::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationBudget {
    pub max_nodes: usize,
    pub max_text_chars: usize,
    pub max_collection_items: usize,
}
impl Default for ObservationBudget { fn default() -> Self { Self { max_nodes: 300, max_text_chars: 24_000, max_collection_items: 40 } } }

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
    pub children: Vec<ObservedElement>,
    pub collapsed_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub schema: u32,
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
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationDiff {
    pub schema: u32,
    pub from_revision: u64,
    pub to_revision: u64,
    pub appeared: Vec<ElementRef>,
    pub disappeared: Vec<ElementRef>,
    pub changed: Vec<ElementChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementChange {
    pub reference: ElementRef,
    pub fields: Vec<String>,
}

impl Observation {
    pub fn diff(&self, newer: &Observation) -> ObservationDiff {
        let old = flatten(&self.roots);
        let new = flatten(&newer.roots);
        let old_keys: BTreeSet<_> = old.keys().cloned().collect();
        let new_keys: BTreeSet<_> = new.keys().cloned().collect();
        let appeared = new_keys.difference(&old_keys).filter_map(|k| new.get(k).map(|e| e.reference.clone())).collect();
        let disappeared = old_keys.difference(&new_keys).filter_map(|k| old.get(k).map(|e| e.reference.clone())).collect();
        let mut changed = Vec::new();
        for key in old_keys.intersection(&new_keys) {
            let a = old[key]; let b = new[key]; let mut fields = Vec::new();
            if a.name != b.name { fields.push("name".into()); }
            if a.value != b.value { fields.push("value".into()); }
            if a.states != b.states { fields.push("states".into()); }
            if a.capabilities != b.capabilities { fields.push("capabilities".into()); }
            if a.alias != b.alias { fields.push("alias".into()); }
            if !fields.is_empty() { changed.push(ElementChange { reference: b.reference.clone(), fields }); }
        }
        ObservationDiff { schema: 1, from_revision: self.revision, to_revision: newer.revision, appeared, disappeared, changed }
    }
}

pub fn observe_raw(graph: &AccessibilityGraph, budget: ObservationBudget) -> Observation {
    let mut stats = ObservationStats { source_nodes: graph.len(), ..Default::default() };
    let mut remaining = budget.max_nodes;
    let mut roots = Vec::new();
    for root in graph.roots() {
        if let Some(observed) = observe_node(graph, *root, &mut remaining, budget.max_collection_items, &mut stats) { roots.push(observed); }
    }
    if remaining == 0 { stats.truncated = true; }
    Observation { schema: 1, session: graph.session(), revision: graph.revision(), view: None, roots, stats }
}

fn observe_node(graph: &AccessibilityGraph, node: NodeId, remaining: &mut usize, max_items: usize, stats: &mut ObservationStats) -> Option<ObservedElement> {
    if *remaining == 0 { stats.truncated = true; return None; }
    *remaining -= 1; stats.exposed_nodes += 1;
    let element = graph.get(node)?;
    let mut states = element.states.iter().filter_map(|(s,v)| (v == StateValue::True).then_some(s)).collect::<Vec<_>>(); states.sort();
    let mut capabilities = element.capabilities.iter().copied().collect::<Vec<_>>(); capabilities.sort();
    let alias = graph.aliases().iter().find_map(|(a,n)| (*n == node).then_some(a.clone()));
    let mut children = Vec::new();
    let child_count = element.children.len();
    for child in element.children.iter().take(max_items) {
        if let Some(v) = observe_node(graph, *child, remaining, max_items, stats) { children.push(v); }
    }
    let collapsed_count = (child_count > children.len()).then_some(child_count - children.len());
    Some(ObservedElement { reference: graph.element_ref_for(node), role: element.role, name: element.name.clone(), value: element.value.as_ref().map(PropertyValue::display_text), states, capabilities, alias, importance: "normal".into(), children, collapsed_count })
}

fn flatten<'a>(roots: &'a [ObservedElement]) -> BTreeMap<String, &'a ObservedElement> {
    fn walk<'a>(node: &'a ObservedElement, out: &mut BTreeMap<String, &'a ObservedElement>) { out.insert(node.reference.opaque.clone(), node); for child in &node.children { walk(child, out); } }
    let mut out = BTreeMap::new(); for root in roots { walk(root, &mut out); } out
}
