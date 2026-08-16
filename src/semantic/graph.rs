use super::model::*;
use super::selector::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub schema: u32,
    pub session: SessionId,
    pub revision: u64,
    pub generation: u64,
    pub roots: Vec<NodeId>,
    pub nodes: Vec<ElementSnapshot>,
}

#[derive(Debug, Clone)]
pub struct AccessibilityGraph {
    session: SessionId,
    revision: u64,
    generation: u64,
    next_node: NodeId,
    roots: Vec<NodeId>,
    nodes: BTreeMap<NodeId, ElementSnapshot>,
    native_index: BTreeMap<String, NodeId>,
    public_index: BTreeMap<String, NodeId>,
    aliases: BTreeMap<String, NodeId>,
}

impl AccessibilityGraph {
    pub fn new(session: SessionId) -> Self {
        Self { session, revision: 0, generation: 1, next_node: 1, roots: Vec::new(), nodes: BTreeMap::new(), native_index: BTreeMap::new(), public_index: BTreeMap::new(), aliases: BTreeMap::new() }
    }

    pub fn session(&self) -> SessionId { self.session }
    pub fn revision(&self) -> u64 { self.revision }
    pub fn generation(&self) -> u64 { self.generation }
    pub fn roots(&self) -> &[NodeId] { &self.roots }
    pub fn len(&self) -> usize { self.nodes.len() }
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }
    pub fn get(&self, node: NodeId) -> Option<&ElementSnapshot> { self.nodes.get(&node) }
    pub fn get_mut(&mut self, node: NodeId) -> Option<&mut ElementSnapshot> { self.nodes.get_mut(&node) }
    pub fn iter(&self) -> impl Iterator<Item = (&NodeId, &ElementSnapshot)> { self.nodes.iter() }
    pub fn node_ids(&self) -> Vec<NodeId> { self.nodes.keys().copied().collect() }
    pub fn find_native(&self, identity: &NativeIdentity) -> Option<NodeId> { self.native_index.get(&native_identity_key(identity)).copied() }
    pub fn set_roots(&mut self, roots: Vec<NodeId>) { self.roots = roots; self.revision = self.revision.saturating_add(1); }

    pub fn clear(&mut self) {
        self.nodes.clear(); self.roots.clear(); self.native_index.clear(); self.public_index.clear(); self.aliases.clear();
        self.generation = self.generation.saturating_add(1); self.revision = self.revision.saturating_add(1); self.next_node = 1;
    }

    pub fn allocate_node(&mut self) -> NodeId { let id = self.next_node; self.next_node += 1; id }

    pub fn insert(&mut self, mut snapshot: ElementSnapshot) -> NodeId {
        if snapshot.node == 0 { snapshot.node = self.allocate_node(); }
        self.next_node = self.next_node.max(snapshot.node + 1);
        let node = snapshot.node;
        if snapshot.parent.is_none() && !self.roots.contains(&node) { self.roots.push(node); }
        self.native_index.insert(native_identity_key(&snapshot.identity.native), node);
        let r = self.element_ref_for(node); self.public_index.insert(r.opaque, node);
        self.nodes.insert(node, snapshot);
        self.revision = self.revision.saturating_add(1);
        node
    }

    pub fn replace_snapshot(&mut self, snapshot: ElementSnapshot) -> NodeId {
        let key = native_identity_key(&snapshot.identity.native);
        let node = self.native_index.get(&key).copied().unwrap_or(snapshot.node);
        let mut snapshot = snapshot;
        snapshot.node = if node == 0 { self.allocate_node() } else { node };
        self.nodes.insert(snapshot.node, snapshot.clone());
        self.native_index.insert(key, snapshot.node);
        self.public_index.insert(self.element_ref_for(snapshot.node).opaque, snapshot.node);
        self.revision = self.revision.saturating_add(1);
        snapshot.node
    }

    pub fn remove(&mut self, node: NodeId) -> Option<ElementSnapshot> {
        let removed = self.nodes.remove(&node)?;
        self.native_index.remove(&native_identity_key(&removed.identity.native));
        self.public_index.retain(|_, n| *n != node);
        self.aliases.retain(|_, n| *n != node);
        self.roots.retain(|n| *n != node);
        if let Some(parent) = removed.parent { if let Some(p) = self.nodes.get_mut(&parent) { p.children.retain(|n| *n != node); } }
        self.revision = self.revision.saturating_add(1);
        Some(removed)
    }

    pub fn element_ref_for(&self, node: NodeId) -> ElementRef {
        let seed = self.session.0 ^ node.rotate_left(17) ^ self.generation.rotate_left(33);
        ElementRef { session: self.session, generation: self.generation, opaque: format!("e_{}", base36(mix64(seed))) }
    }

    pub fn resolve_ref(&self, element_ref: &ElementRef) -> Result<NodeId, String> {
        if element_ref.session != self.session { return Err("element reference belongs to another session".into()); }
        if element_ref.generation != self.generation { return Err("stale element reference; graph generation changed".into()); }
        self.public_index.get(&element_ref.opaque).copied().ok_or_else(|| "unknown or stale element reference".into())
    }

    pub fn resolve_opaque(&self, opaque: &str) -> Option<NodeId> { self.public_index.get(opaque).copied() }
    pub fn bind_alias(&mut self, alias: impl Into<String>, node: NodeId) { self.aliases.insert(alias.into().trim_start_matches('$').to_string(), node); }
    pub fn clear_aliases(&mut self) { self.aliases.clear(); }
    pub fn resolve_alias(&self, alias: &str) -> Option<NodeId> { self.aliases.get(alias.trim_start_matches('$')).copied() }
    pub fn aliases(&self) -> &BTreeMap<String, NodeId> { &self.aliases }

    pub fn descendants(&self, node: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        if let Some(root) = self.nodes.get(&node) { queue.extend(root.children.iter().copied()); }
        while let Some(current) = queue.pop_front() {
            result.push(current);
            if let Some(element) = self.nodes.get(&current) { queue.extend(element.children.iter().copied()); }
        }
        result
    }

    pub fn ancestors(&self, node: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut current = self.nodes.get(&node).and_then(|n| n.parent);
        while let Some(id) = current { result.push(id); current = self.nodes.get(&id).and_then(|n| n.parent); }
        result
    }

    pub fn query(&self, selector: &Selector) -> Vec<NodeId> {
        if let Some(alias) = selector.is_alias_only() { return self.resolve_alias(alias).into_iter().collect(); }
        self.nodes.keys().copied().filter(|node| self.matches_selector(*node, selector)).collect()
    }

    pub fn matches_selector(&self, node: NodeId, selector: &Selector) -> bool {
        if selector.steps.is_empty() { return false; }
        self.matches_step(node, selector, selector.steps.len() - 1)
    }

    fn matches_step(&self, node: NodeId, selector: &Selector, step_index: usize) -> bool {
        let step = &selector.steps[step_index];
        if !step.tests.iter().all(|test| self.matches_test(node, test)) { return false; }
        if step_index == 0 { return true; }
        match step.combinator.unwrap_or(Combinator::Descendant) {
            Combinator::Child => self.nodes.get(&node).and_then(|n| n.parent).map(|p| self.matches_step(p, selector, step_index - 1)).unwrap_or(false),
            Combinator::Descendant => self.ancestors(node).into_iter().any(|ancestor| self.matches_step(ancestor, selector, step_index - 1)),
        }
    }

    fn matches_test(&self, node: NodeId, test: &SelectorTest) -> bool {
        let element = match self.nodes.get(&node) { Some(v) => v, None => return false };
        match test {
            SelectorTest::Role(role) => element.role == *role,
            SelectorTest::StableId(id) => element.identity.stable_id.as_deref() == Some(id.as_str()),
            SelectorTest::Alias(alias) => self.resolve_alias(alias) == Some(node),
            SelectorTest::Property(pred) => element.property_text(&pred.name).map(|v| pred.matches(&v)).unwrap_or(false),
            SelectorTest::State(pred) => element.states.get(pred.state) == pred.expected,
            SelectorTest::Capability(cap) => element.capabilities.contains(cap),
            SelectorTest::Functional(function) => self.matches_function(node, function),
        }
    }

    fn matches_function(&self, node: NodeId, function: &FunctionalPredicate) -> bool {
        match function {
            FunctionalPredicate::Is(selectors) => selectors.iter().any(|selector| self.matches_selector(node, selector)),
            FunctionalPredicate::Not(selector) => !self.matches_selector(node, selector),
            FunctionalPredicate::Has(selector) => self.descendants(node).into_iter().any(|child| self.matches_selector(child, selector)),
            FunctionalPredicate::Relation { kind, selector } => self.nodes.get(&node).map(|element| {
                element.relations.iter().filter(|r| &r.kind == kind).flat_map(|r| r.targets.iter().copied()).any(|target| self.matches_selector(target, selector))
            }).unwrap_or(false),
            FunctionalPredicate::Spatial { relation, selector } => {
                let a = match self.nodes.get(&node).and_then(|e| e.geometry) { Some(g) => g.bounds, None => return false };
                self.query(selector).into_iter().any(|other| {
                    let b = match self.nodes.get(&other).and_then(|e| e.geometry) { Some(g) => g.bounds, None => return false };
                    match relation {
                        SpatialRelation::Near => a.distance_to(&b) <= (a.width.max(a.height).max(b.width.max(b.height)) * 2.0).max(100.0),
                        SpatialRelation::Below => a.y >= b.y + b.height && a.overlaps_x(&b),
                        SpatialRelation::Above => a.y + a.height <= b.y && a.overlaps_x(&b),
                        SpatialRelation::Left => a.x + a.width <= b.x && a.overlaps_y(&b),
                        SpatialRelation::Right => a.x >= b.x + b.width && a.overlaps_y(&b),
                        SpatialRelation::Inside => b.contains(&a),
                    }
                })
            }
        }
    }

    pub fn snapshot(&self) -> GraphSnapshot {
        GraphSnapshot { schema: 1, session: self.session, revision: self.revision, generation: self.generation, roots: self.roots.clone(), nodes: self.nodes.values().cloned().collect() }
    }

    pub fn from_snapshot(snapshot: GraphSnapshot) -> Self {
        let mut graph = Self::new(snapshot.session);
        graph.revision = snapshot.revision; graph.generation = snapshot.generation; graph.roots = snapshot.roots;
        for node in snapshot.nodes {
            graph.next_node = graph.next_node.max(node.node + 1);
            graph.native_index.insert(native_identity_key(&node.identity.native), node.node);
            graph.nodes.insert(node.node, node);
        }
        for node in graph.nodes.keys().copied().collect::<Vec<_>>() { graph.public_index.insert(graph.element_ref_for(node).opaque, node); }
        graph
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for root in &self.roots { if !self.nodes.contains_key(root) { errors.push(format!("root {} does not exist", root)); } }
        for (id, node) in &self.nodes {
            if let Some(parent) = node.parent {
                if !self.nodes.contains_key(&parent) { errors.push(format!("node {} has missing parent {}", id, parent)); }
                else if !self.nodes[&parent].children.contains(id) { errors.push(format!("node {} parent {} does not reference child", id, parent)); }
            }
            for child in &node.children {
                if !self.nodes.contains_key(child) { errors.push(format!("node {} has missing child {}", id, child)); }
                else if self.nodes[child].parent != Some(*id) { errors.push(format!("node {} child {} has inconsistent parent", id, child)); }
            }
            for relation in &node.relations { for target in &relation.targets { if !self.nodes.contains_key(target) { errors.push(format!("node {} relation points to missing {}", id, target)); } } }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

pub fn native_identity_key(identity: &NativeIdentity) -> String {
    match identity {
        NativeIdentity::Uia { runtime_id, process_id } => format!("uia:{:?}:{:?}", process_id, runtime_id),
        NativeIdentity::AtSpi { bus_name, object_path } => format!("atspi:{}:{}", bus_name, object_path),
        NativeIdentity::Ax { registry_token, pid } => format!("ax:{}:{}", pid, registry_token),
        NativeIdentity::Synthetic { key } => format!("synthetic:{}", key),
    }
}

fn mix64(mut x: u64) -> u64 { x ^= x >> 30; x = x.wrapping_mul(0xbf58476d1ce4e5b9); x ^= x >> 27; x = x.wrapping_mul(0x94d049bb133111eb); x ^ (x >> 31) }
fn base36(mut value: u64) -> String { const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"; if value == 0 { return "0".into(); } let mut out = Vec::new(); while value > 0 { out.push(DIGITS[(value % 36) as usize] as char); value /= 36; } out.iter().rev().collect() }

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: NodeId, role: Role, name: &str, parent: Option<NodeId>) -> ElementSnapshot {
        ElementSnapshot { node: id, identity: ElementIdentity { native: NativeIdentity::Synthetic { key: format!("n{id}") }, stable_id: None }, role,
            native_role: NativeRole { backend: BackendKind::Mock, role: role.as_str().into(), subrole: None, numeric_id: None }, name: Some(name.into()),
            description: None, value: None, states: StateSet::default(), capabilities: BTreeSet::new(), actions: vec![], geometry: None, parent,
            children: vec![], relations: vec![], properties: BTreeMap::new(), native: NativeMetadata::default() }
    }

    #[test]
    fn structural_queries_work() {
        let mut g = AccessibilityGraph::new(SessionId::new());
        let mut root = node(1, Role::Toolbar, "Main", None); root.children.push(2); g.insert(root);
        g.insert(node(2, Role::Button, "Save", Some(1)));
        let q = Selector::parse("@toolbar > @button[name=\"Save\"]").unwrap();
        assert_eq!(g.query(&q), vec![2]);
    }

    #[test]
    fn refs_are_generation_scoped() {
        let mut g = AccessibilityGraph::new(SessionId::new()); g.insert(node(1, Role::Button, "Save", None));
        let r = g.element_ref_for(1); assert_eq!(g.resolve_ref(&r).unwrap(), 1); g.clear(); assert!(g.resolve_ref(&r).is_err());
    }
}
