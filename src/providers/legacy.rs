use crate::rpc::types::UiaElement;
use crate::semantic::*;
use std::collections::{BTreeMap, BTreeSet};

pub struct LegacyAccessibilityBackend {
    window: String,
    depth: u32,
    cache: BTreeMap<String, UiaElement>,
    root_key: Option<String>,
}

impl LegacyAccessibilityBackend {
    pub fn new(window: impl Into<String>) -> Self { Self { window: window.into(), depth: 16, cache: BTreeMap::new(), root_key: None } }
    pub fn with_depth(mut self, depth: u32) -> Self { self.depth = depth; self }

    fn reload(&mut self) -> Result<(), String> {
        let tree = crate::ops::dump_tree(&self.window, self.depth).map_err(|e|e.to_string())?;
        self.cache.clear();
        let root = cache_tree(&tree, &mut self.cache, "root");
        self.root_key = Some(root);
        Ok(())
    }

    fn key(identity: &NativeIdentity) -> Result<&str,String> {
        match identity { NativeIdentity::Synthetic { key } => Ok(key), _ => Err("legacy adapter received a non-legacy identity".into()) }
    }

    fn element(&self, identity: &NativeIdentity) -> Result<&UiaElement,String> {
        let key=Self::key(identity)?; self.cache.get(key).ok_or_else(||format!("legacy element '{}' is stale",key))
    }

    fn selector_for(&self, element:&UiaElement)->String {
        if !element.automation_id.is_empty() { format!("#{}",element.automation_id) }
        else if !element.name.is_empty() { element.name.clone() }
        else { element.control_type.clone() }
    }
}

impl AccessibilityBackend for LegacyAccessibilityBackend {
    fn kind(&self) -> BackendKind { platform_kind() }

    fn roots(&mut self) -> Result<Vec<BackendRoot>,String> {
        self.reload()?;
        let key=self.root_key.clone().ok_or_else(||"accessibility tree has no root".to_string())?;
        let name=self.cache.get(&key).map(|e|e.name.clone()).filter(|s|!s.is_empty());
        Ok(vec![BackendRoot{native:NativeIdentity::Synthetic{key},name,pid:None}])
    }

    fn inspect(&mut self, identity:&NativeIdentity, _mask:InspectMask)->Result<ElementSnapshot,String>{
        let key=Self::key(identity)?.to_string(); let element=self.element(identity)?.clone(); Ok(convert_element(&key,&element,self.kind()))
    }

    fn children(&mut self, identity:&NativeIdentity, range:ChildRange)->Result<Vec<NativeIdentity>,String>{
        let element=self.element(identity)?; let start=range.offset.min(element.children.len()); let end=(start+range.limit).min(element.children.len());
        Ok(element.children[start..end].iter().enumerate().map(|(offset,c)|NativeIdentity::Synthetic{key:element_key(c,&format!("child{}",start+offset))}).collect())
    }

    fn perform(&mut self, identity:&NativeIdentity, request:ActionRequest)->Result<BackendActionResult,String>{
        let element=self.element(identity)?.clone(); let selector=self.selector_for(&element);
        let pattern=match request.action { SemanticAction::Activate=>"invoke",SemanticAction::SetValue|SemanticAction::SetText|SemanticAction::ReplaceText=>"set-value",SemanticAction::Toggle=>"toggle",SemanticAction::Select=>"select",SemanticAction::Expand=>"expand",SemanticAction::Collapse=>"collapse",SemanticAction::Increment=>"increment",SemanticAction::Decrement=>"decrement",SemanticAction::ShowMenu=>"invoke",SemanticAction::Confirm=>"invoke",SemanticAction::Cancel=>"invoke",SemanticAction::ScrollIntoView=>"scroll-into-view",SemanticAction::Focus=>"focus",SemanticAction::RaiseWindow=>"invoke"};
        let owned_value=match request.value {ActionValue::Text(v)=>Some(v),ActionValue::Bool(v)=>Some(v.to_string()),ActionValue::Number(v)=>Some(v.to_string()),ActionValue::Range(v)=>Some(format!("{}..{}",v.start,v.end)),ActionValue::None=>None};
        let result=crate::ops::invoke_pattern(&self.window,&selector,pattern,owned_value.as_deref()).map_err(|e|e.to_string())?;
        Ok(BackendActionResult{success:result.success,native_action:Some(pattern.into()),message:result.error})
    }

    fn plan_query(&self,selector:&crate::semantic::Selector)->NativeQueryPlan{
        let description=match self.kind(){BackendKind::WindowsUia=>"UIA condition/cache pushdown available after native-v2 migration",BackendKind::LinuxAtSpi=>"AT-SPI Collection pushdown available after native-v2 migration",BackendKind::MacAx=>"AX early-prune/batched-attribute traversal",BackendKind::Mock=>"local semantic scan"};
        NativeQueryPlan{backend:self.kind(),description:description.into(),pushed_predicates:Vec::new(),residual_selector:selector.source.clone(),exact:false}
    }
}

pub struct LegacyInputBackend { window:String }
impl LegacyInputBackend { pub fn new(window:impl Into<String>)->Self{Self{window:window.into()}} }
impl InputBackend for LegacyInputBackend {
    fn click(&mut self,point:Point,button:MouseButton)->Result<(),String>{let kind=match button{MouseButton::Left=>"left",MouseButton::Right=>"right",MouseButton::Middle=>"middle"};crate::ops::click(&self.window,kind,Some((point.x.round() as i32,point.y.round() as i32)),None).map_err(|e|e.to_string())}
    fn double_click(&mut self,point:Point,_button:MouseButton)->Result<(),String>{crate::ops::click(&self.window,"double",Some((point.x.round() as i32,point.y.round() as i32)),None).map_err(|e|e.to_string())}
    fn type_text(&mut self,text:&str)->Result<(),String>{crate::ops::type_text(&self.window,text,None).map_err(|e|e.to_string())}
    fn keys(&mut self,keys:&str)->Result<(),String>{crate::ops::send_keys(&self.window,keys).map_err(|e|e.to_string())}
    fn scroll(&mut self,_dx:f64,dy:f64,_at:Option<Point>)->Result<(),String>{let direction=if dy>=0.0{"up"}else{"down"};crate::ops::scroll(&self.window,direction,dy.abs().round().max(1.0) as i32).map_err(|e|e.to_string())}
}

pub struct LegacyWindowBackend;
impl WindowBackend for LegacyWindowBackend {
    fn list_windows(&mut self)->Result<Vec<WindowInfoV2>,String>{crate::ops::list_windows(None,None).map_err(|e|e.to_string()).map(|ws|ws.into_iter().map(|w|WindowInfoV2{id:w.hwnd,title:w.title,executable:w.executable,pid:w.pid,bounds:Some(Rect{x:w.rect.x as f64,y:w.rect.y as f64,width:w.rect.width as f64,height:w.rect.height as f64}),native_class:w.class_name,bundle_id:None}).collect())}
    fn focus_window(&mut self,_id:&str)->Result<(),String>{Err("focus_window is not exposed by the legacy compatibility layer".into())}
}

fn cache_tree(element:&UiaElement,out:&mut BTreeMap<String,UiaElement>,fallback:&str)->String{let key=element_key(element,fallback);out.insert(key.clone(),element.clone());for(i,child)in element.children.iter().enumerate(){cache_tree(child,out,&format!("{key}/{i}"));}key}
fn element_key(element:&UiaElement,_fallback:&str)->String{
    if !element.id.is_empty(){
        element.id.clone()
    } else if !element.automation_id.is_empty(){
        format!("id:{}",element.automation_id)
    } else {
        format!("legacy:{}:{}:{}:{}:{}:{}", element.control_type, element.name, element.bounds[0], element.bounds[1], element.bounds[2], element.bounds[3])
    }
}

fn convert_element(key:&str,element:&UiaElement,backend:BackendKind)->ElementSnapshot{
    let role=Role::parse(&element.control_type);let mut states=StateSet::default();states.set_bool(State::Enabled,element.is_enabled);states.set_bool(State::Visible,!element.is_offscreen);states.set_bool(State::Offscreen,element.is_offscreen);
    let mut capabilities=BTreeSet::new();let mut actions=Vec::new();
    for pattern in &element.patterns{if let Some((cap,action))=pattern_mapping(pattern){capabilities.insert(cap);if let Some(action)=action{actions.push(ElementAction{semantic:Some(action),native:NativeAction{backend,name:pattern.clone(),index:None},name:pattern.clone(),description:None,key_binding:None});}}}
    let mut properties=BTreeMap::new();if !element.class_name.is_empty(){properties.insert("class".into(),PropertyValue::String(element.class_name.clone()));}if !element.localized_type.is_empty(){properties.insert("localized-type".into(),PropertyValue::String(element.localized_type.clone()));}if !element.automation_id.is_empty(){properties.insert("native.automation-id".into(),PropertyValue::String(element.automation_id.clone()));}
    ElementSnapshot{node:0,identity:ElementIdentity{native:NativeIdentity::Synthetic{key:key.into()},stable_id:(!element.automation_id.is_empty()).then(||element.automation_id.clone())},role,native_role:NativeRole{backend,role:element.control_type.clone(),subrole:None,numeric_id:None},name:(!element.name.is_empty()).then(||element.name.clone()),description:None,value:element.value.clone().map(PropertyValue::String),states,capabilities,actions,geometry:Some(Geometry{bounds:Rect{x:element.bounds[0] as f64,y:element.bounds[1] as f64,width:element.bounds[2] as f64,height:element.bounds[3] as f64},coordinate_space:CoordinateSpace::ScreenPhysical,visibility:if element.is_offscreen{Visibility::Offscreen}else{Visibility::Visible},z_order:None}),parent:None,children:Vec::new(),relations:Vec::new(),properties,native:NativeMetadata{attributes:BTreeMap::new(),interfaces:BTreeSet::new(),actions:element.patterns.clone()}}
}

fn pattern_mapping(pattern:&str)->Option<(CapabilityKind,Option<SemanticAction>)>{Some(match pattern.to_ascii_lowercase().as_str(){"invoke"=>(CapabilityKind::Action,Some(SemanticAction::Activate)),"value"=>(CapabilityKind::Value,Some(SemanticAction::SetValue)),"toggle"=>(CapabilityKind::Toggle,Some(SemanticAction::Toggle)),"expandcollapse"|"expand-collapse"=>(CapabilityKind::ExpandCollapse,Some(SemanticAction::Expand)),"selection"=>(CapabilityKind::Selection,None),"selectionitem"=>(CapabilityKind::SelectionItem,Some(SemanticAction::Select)),"text"|"text2"=>(CapabilityKind::Text,None),"grid"|"table"=>(CapabilityKind::Table,None),"griditem"|"tableitem"=>(CapabilityKind::TableCell,None),"scroll"=>(CapabilityKind::Scroll,None),"scrollitem"=>(CapabilityKind::ScrollItem,Some(SemanticAction::ScrollIntoView)),"window"=>(CapabilityKind::Window,None),"transform"|"transform2"=>(CapabilityKind::Transform,None),"virtualizeditem"=>(CapabilityKind::VirtualizedItem,None),"itemcontainer"=>(CapabilityKind::ItemContainer,None),_=>return None})}

fn platform_kind()->BackendKind{#[cfg(windows)]{return BackendKind::WindowsUia;}#[cfg(target_os="linux")]{return BackendKind::LinuxAtSpi;}#[cfg(target_os="macos")]{return BackendKind::MacAx;}#[allow(unreachable_code)]BackendKind::Mock}
