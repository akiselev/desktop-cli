pub mod protocol;

use crate::packs::{project_with_redaction, ApplicationPack, ProjectionResult};
use crate::semantic::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::Receiver;

pub struct DesktopSession {
    pub id: SessionId,
    graph: AccessibilityGraph,
    accessibility: Box<dyn AccessibilityBackend>,
    input: Option<Box<dyn InputBackend>>,
    pack: Option<ApplicationPack>,
    event_rx: Option<Receiver<AccessibilityEvent>>,
    action_policy: ActionPolicy,
    redaction_policy: RedactionPolicy,
    telemetry: OperationTelemetry,
    max_depth: usize,
    max_children: usize,
}

impl DesktopSession {
    pub fn new(accessibility: Box<dyn AccessibilityBackend>, input: Option<Box<dyn InputBackend>>) -> Self {
        let id = SessionId::new();
        Self { id, graph: AccessibilityGraph::new(id), accessibility, input, pack: None, event_rx: None, action_policy: ActionPolicy::default(), redaction_policy: RedactionPolicy::default(), telemetry: OperationTelemetry::default(), max_depth: 32, max_children: 512 }
    }

    pub fn graph(&self) -> &AccessibilityGraph { &self.graph }
    pub fn graph_mut(&mut self) -> &mut AccessibilityGraph { &mut self.graph }
    pub fn backend_kind(&self) -> BackendKind { self.accessibility.kind() }
    pub fn set_pack(&mut self, pack: Option<ApplicationPack>) { self.pack = pack; self.graph.clear_aliases(); }
    pub fn pack(&self) -> Option<&ApplicationPack> { self.pack.as_ref() }
    pub fn action_policy(&self) -> &ActionPolicy { &self.action_policy }
    pub fn action_policy_mut(&mut self) -> &mut ActionPolicy { &mut self.action_policy }
    pub fn set_action_policy(&mut self, policy: ActionPolicy) { self.action_policy = policy; }
    pub fn redaction_policy_mut(&mut self) -> &mut RedactionPolicy { &mut self.redaction_policy }
    pub fn telemetry(&self) -> &OperationTelemetry { &self.telemetry }
    pub fn set_limits(&mut self, max_depth: usize, max_children: usize) { self.max_depth = max_depth.max(1); self.max_children = max_children.max(1); }

    pub fn refresh(&mut self) -> Result<u64, String> {
        let timer = OperationTimer::start();
        let roots = self.accessibility.roots()?;
        self.telemetry.native_calls += 1;
        let mut seen = BTreeSet::new();
        let mut graph_roots = Vec::new();
        for root in roots {
            let id = self.sync_subtree(&root.native, None, 0, &mut seen)?;
            graph_roots.push(id);
        }
        let existing = self.graph.node_ids();
        for node in existing.into_iter().filter(|n| !seen.contains(n)) { self.graph.remove(node); }
        self.graph.set_roots(graph_roots);
        self.graph.validate().map_err(|errors| errors.join("; "))?;
        timer.finish_into(&mut self.telemetry.backend_micros);
        Ok(self.graph.revision())
    }

    fn sync_subtree(&mut self, native: &NativeIdentity, parent: Option<NodeId>, depth: usize, seen: &mut BTreeSet<NodeId>) -> Result<NodeId, String> {
        if depth > self.max_depth { return Err(format!("accessibility tree exceeded max depth {}", self.max_depth)); }
        let existing = self.graph.find_native(native);
        let mut snapshot = self.accessibility.inspect(native, InspectMask::BASIC)?;
        self.telemetry.native_calls += 1;
        self.telemetry.elements_inspected += 1;
        if let Some(node) = existing { snapshot.node = node; self.telemetry.graph_nodes_updated += 1; } else { self.telemetry.graph_nodes_created += 1; }
        snapshot.parent = parent;
        snapshot.children.clear();
        let node = self.graph.replace_snapshot(snapshot);
        if !seen.insert(node) { return Ok(node); }
        let child_identities = self.accessibility.children(native, ChildRange { offset: 0, limit: self.max_children })?;
        self.telemetry.native_calls += 1;
        self.telemetry.children_loaded += child_identities.len() as u64;
        let mut children = Vec::new();
        for child in child_identities {
            let child_id = self.sync_subtree(&child, Some(node), depth + 1, seen)?;
            if child_id != node { children.push(child_id); }
        }
        if let Some(element) = self.graph.get_mut(node) { element.children = children; }
        Ok(node)
    }

    pub fn query(&self, selector: &str) -> Result<Vec<ElementRef>, String> {
        let selector = Selector::parse(selector)?;
        Ok(self.graph.query(&selector).into_iter().map(|node| self.graph.element_ref_for(node)).collect())
    }

    pub fn explain_query(&self, selector: &str) -> Result<NativeQueryPlan, String> {
        let selector = Selector::parse(selector)?;
        Ok(self.accessibility.plan_query(&selector))
    }

    pub fn observe(&mut self, budget: ObservationBudget) -> Result<ProjectionResult, String> {
        let timer = OperationTimer::start();
        let result = if let Some(pack) = self.pack.clone() { project_with_redaction(&mut self.graph, &pack, budget, &self.redaction_policy)? }
        else {
            let observation = observe_raw_with_redaction(&self.graph, budget, &self.redaction_policy);
            ProjectionResult { observation, active_view: "raw-semantic".into(), aliases: BTreeMap::new(), diagnostics: Vec::new() }
        };
        self.telemetry.projected_nodes += result.observation.stats.exposed_nodes as u64;
        self.telemetry.projection_micros = timer.elapsed().as_micros().min(u64::MAX as u128) as u64;
        Ok(result)
    }

    pub fn perform(&mut self, target: &str, action: SemanticAction, value: ActionValue, allow_input_fallback: bool) -> Result<SemanticActionResult, String> {
        let before = self.observe(ObservationBudget::default())?.observation;
        let node = self.resolve_action_target(target)?;
        let element_ref = self.graph.element_ref_for(node);
        let mut result = {
            let mut router = ActionRouter { graph: &self.graph, accessibility: self.accessibility.as_mut(), input: self.input.as_deref_mut(), policy: &self.action_policy };
            router.perform(SemanticActionRequest { target: element_ref, action, value, allow_input_fallback })?
        };
        let _ = self.refresh();
        let after = self.observe(ObservationBudget::default())?.observation;
        result.before_revision = before.revision;
        result.after_revision = after.revision;
        result.observation = Some(before.diff(&after));
        Ok(result)
    }

    fn resolve_action_target(&self, target: &str) -> Result<NodeId, String> {
        if let Some(alias) = target.strip_prefix('$') { return self.graph.resolve_alias(alias).ok_or_else(|| format!("alias '{}' does not resolve", target)); }
        if let Some(node) = self.graph.resolve_opaque(target) { return Ok(node); }
        let selector = Selector::parse(target)?;
        let matches = self.graph.query(&selector);
        if matches.len() != 1 { return Err(format!("action target '{}' resolved to {} elements", target, matches.len())); }
        Ok(matches[0])
    }

    pub fn perform_pack_action(&mut self, name: &str, value: ActionValue) -> Result<SemanticActionResult, String> {
        let pack = self.pack.clone().ok_or_else(|| "no pack is active".to_string())?;
        let action = pack.stylesheet.actions.iter().find(|a| a.name == name).cloned().ok_or_else(|| format!("pack action '{}' does not exist", name))?;
        let _ = self.observe(ObservationBudget::default())?;
        match self.perform(&action.target, action.perform, value.clone(), false) {
            Ok(result) if result.success => Ok(result),
            Ok(result) => self.perform_pack_keyboard_fallback(action, value, result.message.unwrap_or_else(|| "native action reported failure".into())),
            Err(primary) => self.perform_pack_keyboard_fallback(action, value, primary),
        }
    }

    fn perform_pack_keyboard_fallback(&mut self, action: crate::packs::PackAction, _value: ActionValue, primary: String) -> Result<SemanticActionResult, String> {
        let keys = action.fallback_keys.ok_or(primary.clone())?;
        let before = self.observe(ObservationBudget::default())?.observation;
        let node = self.resolve_action_target(&action.target)?;
        let element = self.graph.get(node).ok_or_else(|| "pack target disappeared".to_string())?;
        self.action_policy.check(element, action.perform, false, true)?;
        {
            let input = self.input.as_deref_mut().ok_or_else(|| format!("{}; pack fallback '{}' requires an input backend", primary, keys))?;
            input.keys(&keys)?;
        }
        let _ = self.refresh();
        let after = self.observe(ObservationBudget::default())?.observation;
        Ok(SemanticActionResult { schema_version: 2, success: true, target: self.graph.element_ref_for(node), action: action.perform, route: "pack-keyboard-fallback".into(), before_revision: before.revision, after_revision: after.revision, observation: Some(before.diff(&after)), warnings: vec![format!("native route failed: {}", primary)], message: Some(format!("used policy-approved pack fallback {}", keys)) })
    }

    pub fn start_events(&mut self) -> Result<(), String> {
        let first_root = self.accessibility.roots()?.into_iter().next().ok_or_else(|| "no accessibility root".to_string())?;
        let interests = [EventKind::Created, EventKind::Destroyed, EventKind::ChildrenChanged, EventKind::PropertyChanged, EventKind::StateChanged, EventKind::FocusChanged, EventKind::SelectionChanged, EventKind::TextChanged, EventKind::GeometryChanged, EventKind::WindowCreated, EventKind::WindowDestroyed].into_iter().collect();
        self.event_rx = Some(self.accessibility.subscribe(&first_root.native, &interests)?);
        Ok(())
    }

    pub fn apply_pending_events(&mut self, max_events: usize) -> Result<usize, String> {
        let Some(rx) = self.event_rx.as_ref() else { return Ok(0); };
        let mut count = 0;
        while count < max_events { match rx.try_recv() { Ok(_) => count += 1, Err(_) => break } }
        if count > 0 { self.refresh()?; }
        Ok(count)
    }
}
