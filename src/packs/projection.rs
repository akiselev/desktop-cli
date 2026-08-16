use super::model::*;
use crate::semantic::{AccessibilityGraph, CapabilityKind, ElementRef, NodeId, Observation, ObservationBudget, ObservationStats, ObservedElement, PropertyValue, Role, State, StateValue};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionResult {
    pub observation: Observation,
    pub active_view: String,
    pub aliases: BTreeMap<String, ElementRef>,
    pub diagnostics: Vec<PackDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleExplanation {
    pub node: ElementRef,
    pub matched_rules: Vec<String>,
    pub final_projection: ProjectionOp,
    pub alias: Option<String>,
    pub importance: Importance,
    pub critical_overlay: bool,
}

#[derive(Debug, Clone)]
struct EffectiveRule {
    projection: ProjectionOp,
    alias: Option<String>,
    importance: Importance,
    max_items: Option<usize>,
    max_depth: Option<usize>,
    matched: Vec<String>,
}

pub fn choose_view<'a>(graph: &AccessibilityGraph, pack: &'a ApplicationPack) -> Result<&'a PackView, String> {
    let mut active = pack.stylesheet.views.iter().filter(|v| v.when.as_ref().map(|s| !graph.query(s).is_empty()).unwrap_or(false)).collect::<Vec<_>>();
    active.sort_by_key(|v| (v.priority, v.order as i32));
    if let Some(view) = active.last() { return Ok(view); }
    pack.stylesheet.views.iter().find(|v| v.name == pack.manifest.default_view).or_else(|| pack.stylesheet.views.first()).ok_or_else(|| "pack contains no views".into())
}

pub fn project(graph: &mut AccessibilityGraph, pack: &ApplicationPack, budget: ObservationBudget) -> Result<ProjectionResult, String> {
    let view = choose_view(graph, pack)?.clone();
    graph.clear_aliases();
    let rules = collect_rules(pack, &view)?;
    let mut diagnostics = Vec::new();

    for target in &pack.stylesheet.targets {
        let mut resolved = Vec::new();
        for selector in &target.matches {
            let found = graph.query(selector);
            if !found.is_empty() { resolved = found; break; }
        }
        if resolved.is_empty() {
            diagnostics.push(PackDiagnostic { level: DiagnosticLevel::Warning, message: format!("target '{}' did not resolve", target.name), span: Some(target.span.clone()) });
        } else if resolved.len() > 1 && !target.multi {
            diagnostics.push(PackDiagnostic { level: DiagnosticLevel::Warning, message: format!("target '{}' is ambiguous ({} matches)", target.name, resolved.len()), span: Some(target.span.clone()) });
        } else if !target.multi {
            graph.bind_alias(&target.name, resolved[0]);
        }
    }

    let node_ids = graph.iter().map(|(id,_)| *id).collect::<Vec<_>>();
    for node in node_ids {
        for rule in &rules {
            if let Some(alias) = &rule.alias {
                if graph.matches_selector(node, &rule.selector) {
                    if let Some(existing) = graph.resolve_alias(alias) {
                        if existing != node {
                            diagnostics.push(PackDiagnostic { level: DiagnosticLevel::Warning, message: format!("alias '${}' resolved more than once", alias), span: Some(rule.span.clone()) });
                        }
                    } else { graph.bind_alias(alias, node); }
                }
            }
        }
    }

    let mut stats = ObservationStats { source_nodes: graph.len(), ..Default::default() };
    let mut remaining = budget.max_nodes;
    let mut roots = Vec::new();
    for root in graph.roots().to_vec() {
        roots.extend(project_node(graph, root, 0, &rules, view.default_projection, &mut remaining, budget, &mut stats));
    }
    if remaining == 0 { stats.truncated = true; }
    let aliases = graph.aliases().iter().map(|(a,n)|(a.clone(),graph.element_ref_for(*n))).collect();
    let observation = Observation { schema: 1, session: graph.session(), revision: graph.revision(), view: Some(view.name.clone()), roots, stats };
    Ok(ProjectionResult { observation, active_view: view.name, aliases, diagnostics })
}

pub fn explain(graph: &AccessibilityGraph, pack: &ApplicationPack, node: NodeId) -> Result<RuleExplanation, String> {
    let view = choose_view(graph, pack)?;
    let rules = collect_rules(pack, view)?;
    let effective = effective_rule(graph, node, &rules, view.default_projection);
    Ok(RuleExplanation { node: graph.element_ref_for(node), matched_rules: effective.matched, final_projection: effective.projection, alias: effective.alias, importance: effective.importance, critical_overlay: is_critical(graph, node) })
}

fn collect_rules<'a>(pack: &'a ApplicationPack, view: &'a PackView) -> Result<Vec<&'a PackRule>, String> {
    fn add<'a>(pack: &'a ApplicationPack, view: &'a PackView, visiting: &mut BTreeSet<String>, out: &mut Vec<&'a PackRule>) -> Result<(),String> {
        if !visiting.insert(view.name.clone()) { return Err(format!("view inheritance cycle at '{}'", view.name)); }
        for base in &view.uses {
            let base_view = pack.stylesheet.views.iter().find(|v| &v.name == base).ok_or_else(|| format!("missing base view '{}'", base))?;
            add(pack, base_view, visiting, out)?;
        }
        out.extend(view.rules.iter()); visiting.remove(&view.name); Ok(())
    }
    let mut out=Vec::new(); add(pack, view, &mut BTreeSet::new(), &mut out)?; Ok(out)
}

fn effective_rule(graph: &AccessibilityGraph, node: NodeId, rules: &[&PackRule], default_projection: ProjectionOp) -> EffectiveRule {
    let mut result = EffectiveRule { projection: default_projection, alias: None, importance: Importance::Normal, max_items: None, max_depth: None, matched: Vec::new() };
    for rule in rules {
        if graph.matches_selector(node, &rule.selector) {
            result.matched.push(rule.selector.source.clone());
            if let Some(v)=rule.projection { result.projection=v; }
            if let Some(v)=&rule.alias { result.alias=Some(v.clone()); }
            if let Some(v)=rule.importance { result.importance=v; }
            if rule.max_items.is_some(){result.max_items=rule.max_items;}
            if rule.max_depth.is_some(){result.max_depth=rule.max_depth;}
        }
    }
    if is_critical(graph,node) { result.projection=ProjectionOp::Keep; result.importance=Importance::Critical; }
    result
}

fn project_node(graph:&AccessibilityGraph,node:NodeId,depth:usize,rules:&[&PackRule],default_projection:ProjectionOp,remaining:&mut usize,budget:ObservationBudget,stats:&mut ObservationStats)->Vec<ObservedElement>{
    let effective=effective_rule(graph,node,rules,default_projection);
    let element=match graph.get(node){Some(e)=>e,None=>return vec![]};
    let max_items=effective.max_items.unwrap_or(budget.max_collection_items);
    if effective.max_depth.map(|d|depth>d).unwrap_or(false){stats.pruned_nodes+=1;return vec![];}
    match effective.projection {
        ProjectionOp::Prune => {
            let mut critical=Vec::new();
            for child in &element.children { if subtree_has_critical(graph,*child) { critical.extend(project_node(graph,*child,depth,rules,default_projection,remaining,budget,stats)); } }
            if critical.is_empty(){stats.pruned_nodes+=1;} critical
        }
        ProjectionOp::Flatten => {
            let mut out=Vec::new(); for child in element.children.iter().take(max_items){out.extend(project_node(graph,*child,depth,rules,default_projection,remaining,budget,stats));} out
        }
        ProjectionOp::Keep | ProjectionOp::Collapse => {
            if *remaining==0 {stats.truncated=true;return vec![];} *remaining-=1;stats.exposed_nodes+=1;
            let collapsed=effective.projection==ProjectionOp::Collapse;
            if collapsed{stats.collapsed_nodes+=1;}
            let children=if collapsed{Vec::new()}else{let mut v=Vec::new();for child in element.children.iter().take(max_items){v.extend(project_node(graph,*child,depth+1,rules,default_projection,remaining,budget,stats));}v};
            let mut states=element.states.iter().filter_map(|(s,v)|(v==StateValue::True).then_some(s)).collect::<Vec<_>>();states.sort();
            let mut caps=element.capabilities.iter().copied().collect::<Vec<CapabilityKind>>();caps.sort();
            let alias=effective.alias.or_else(||graph.aliases().iter().find_map(|(a,n)|(*n==node).then_some(a.clone())));
            vec![ObservedElement{reference:graph.element_ref_for(node),role:element.role,name:element.name.clone(),value:element.value.as_ref().map(PropertyValue::display_text),states,capabilities:caps,alias,importance:format!("{:?}",effective.importance).to_ascii_lowercase(),children,collapsed_count:collapsed.then_some(graph.descendants(node).len())}]
        }
    }
}

fn is_critical(graph:&AccessibilityGraph,node:NodeId)->bool{graph.get(node).map(|e|e.role==Role::Alert||e.states.is_true(State::Modal)||e.states.is_true(State::Focused)).unwrap_or(false)}
fn subtree_has_critical(graph:&AccessibilityGraph,node:NodeId)->bool{is_critical(graph,node)||graph.get(node).map(|e|e.children.iter().any(|c|subtree_has_critical(graph,*c))).unwrap_or(false)}

pub fn resolve_target(graph:&AccessibilityGraph,pack:&ApplicationPack,name:&str)->Result<Vec<NodeId>,String>{
    if let Some(alias)=name.strip_prefix('$'){if let Some(node)=graph.resolve_alias(alias){return Ok(vec![node]);}}
    let target=pack.stylesheet.targets.iter().find(|t|t.name==name.trim_start_matches('$')).ok_or_else(||format!("unknown target '{}'",name))?;
    for selector in &target.matches{let found=graph.query(selector);if !found.is_empty(){if !target.multi&&found.len()!=1{return Err(format!("target '{}' is ambiguous",name));}return Ok(found);}}
    Err(format!("target '{}' did not resolve",name))
}

#[cfg(test)] mod tests {
    use super::*; use crate::packs::{parse_manifest,parse_stylesheet}; use crate::semantic::*; use std::collections::{BTreeMap,BTreeSet};
    fn elem(id:u64,role:Role,name:&str,parent:Option<u64>)->ElementSnapshot{ElementSnapshot{node:id,identity:ElementIdentity{native:NativeIdentity::Synthetic{key:format!("n{id}")},stable_id:None},role,native_role:NativeRole{backend:BackendKind::Mock,role:role.as_str().into(),subrole:None,numeric_id:None},name:Some(name.into()),description:None,value:None,states:StateSet::default(),capabilities:BTreeSet::new(),actions:vec![],geometry:None,parent,children:vec![],relations:vec![],properties:BTreeMap::new(),native:NativeMetadata::default()}}
    #[test]fn whitelist_projection(){let mut g=AccessibilityGraph::new(SessionId::new());let mut r=elem(1,Role::Panel,"root",None);r.children=vec![2,3];g.insert(r);g.insert(elem(2,Role::Button,"Save",Some(1)));g.insert(elem(3,Role::Button,"Noise",Some(1)));let pack=ApplicationPack{manifest:parse_manifest("schema=1\nid=\"x\"\nname=\"X\"\n[view]\ndefault=\"workspace\"").unwrap(),stylesheet:parse_stylesheet("@view workspace { default-projection: prune; @panel { projection: flatten; } @button[name=\"Save\"] { projection: keep; alias: save; } }").unwrap()};let p=project(&mut g,&pack,ObservationBudget::default()).unwrap();assert_eq!(p.observation.stats.exposed_nodes,1);assert!(p.aliases.contains_key("save"));}
}
