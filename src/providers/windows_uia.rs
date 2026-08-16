#![cfg(windows)]

use crate::providers::windows::role_from_uia;
use crate::semantic::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use uiautomation::patterns::{
    UIExpandCollapsePattern, UIGridItemPattern, UIGridPattern, UIInvokePattern, UIRangeValuePattern,
    UIScrollItemPattern, UIScrollPattern, UISelectionItemPattern, UISelectionPattern, UITableItemPattern,
    UITablePattern, UITextPattern, UITogglePattern, UITransformPattern, UIValuePattern,
    UIVirtualizedItemPattern, UIWindowPattern, UIPatternType,
};
use uiautomation::types::{ExpandCollapseState, Handle, ToggleState, TreeScope, UIProperty};
use uiautomation::{UIAutomation, UIElement, UITreeWalker};

macro_rules! cached_pattern {
    ($element:expr, $ty:ty) => {
        $element.get_cached_pattern::<$ty>().or_else(|_| $element.get_pattern::<$ty>()).ok()
    };
}
macro_rules! add_capability {
    ($element:expr, $ty:ty, $cap:expr, $caps:expr) => {
        if cached_pattern!($element, $ty).is_some() { $caps.insert($cap); }
    };
}

pub struct WindowsUiaBackend { tx: Sender<Command> }

impl WindowsUiaBackend {
    pub fn new(window: &str) -> Result<Self, String> {
        let hwnd = parse_hwnd_value(window)?;
        let (tx, rx) = mpsc::channel();
        thread::Builder::new().name("desktop-cli-uia".into()).spawn(move || worker_main(hwnd, rx)).map_err(|e| format!("failed to spawn UIA actor: {e}"))?;
        let backend = Self { tx };
        backend.call(|reply| Command::Ping { reply })?;
        Ok(backend)
    }

    fn call<T: Send + 'static>(&self, make: impl FnOnce(Sender<Result<T, String>>) -> Command) -> Result<T, String> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(make(tx)).map_err(|_| "UIA actor disconnected".to_string())?;
        rx.recv().map_err(|_| "UIA actor disconnected".to_string())?
    }
}

impl AccessibilityBackend for WindowsUiaBackend {
    fn kind(&self) -> BackendKind { BackendKind::WindowsUia }
    fn roots(&mut self) -> Result<Vec<BackendRoot>, String> { self.call(|reply| Command::Roots { reply }) }
    fn inspect(&mut self, identity: &NativeIdentity, mask: InspectMask) -> Result<ElementSnapshot, String> { let identity=identity.clone(); self.call(|reply| Command::Inspect { identity, mask, reply }) }
    fn children(&mut self, identity: &NativeIdentity, range: ChildRange) -> Result<Vec<NativeIdentity>, String> { let identity=identity.clone(); self.call(|reply| Command::Children { identity, range, reply }) }
    fn perform(&mut self, identity: &NativeIdentity, request: ActionRequest) -> Result<BackendActionResult, String> { let identity=identity.clone(); self.call(|reply| Command::Perform { identity, request, reply }) }
    fn query_text(&mut self, identity: &NativeIdentity, query: TextQuery) -> Result<TextResult, String> { let identity=identity.clone(); self.call(|reply| Command::Text { identity, query, reply }) }

    fn plan_query(&self, selector: &Selector) -> NativeQueryPlan {
        let mut pushed=Vec::new();let mut residual=Vec::new();
        for step in &selector.steps { for test in &step.tests { match test {
            SelectorTest::Role(role)=>pushed.push(format!("ControlType={}",role.as_str())),
            SelectorTest::StableId(id)=>pushed.push(format!("AutomationId={id}")),
            SelectorTest::Property(p) if matches!(p.name.as_str(),"name"|"class"|"native.automation-id")=>pushed.push(format!("{} {:?} {}",p.name,p.op,p.value)),
            SelectorTest::State(p) if matches!(p.state,State::Enabled|State::Focused|State::Offscreen)=>pushed.push(format!("state.{:?}={:?}",p.state,p.expected)),
            other=>residual.push(format!("{:?}",other)),
        }}}
        NativeQueryPlan{backend:BackendKind::WindowsUia,description:"UIA actor uses cached control-tree traversal; simple property predicates are native-pushdown eligible and remaining predicates are evaluated by the semantic graph".into(),pushed_predicates:pushed,residual_selector:if residual.is_empty(){String::new()}else{selector.source.clone()},exact:residual.is_empty()}
    }
}

enum Command {
    Ping { reply: Sender<Result<(),String>> },
    Roots { reply: Sender<Result<Vec<BackendRoot>,String>> },
    Inspect { identity:NativeIdentity, mask:InspectMask, reply:Sender<Result<ElementSnapshot,String>> },
    Children { identity:NativeIdentity, range:ChildRange, reply:Sender<Result<Vec<NativeIdentity>,String>> },
    Perform { identity:NativeIdentity, request:ActionRequest, reply:Sender<Result<BackendActionResult,String>> },
    Text { identity:NativeIdentity, query:TextQuery, reply:Sender<Result<TextResult,String>> },
}

struct Worker { _automation:UIAutomation, walker:UITreeWalker, cache:uiautomation::core::UICacheRequest, root:UIElement, elements:BTreeMap<String,UIElement> }

fn worker_main(hwnd:isize,rx:Receiver<Command>){
    let mut state=Worker::new(hwnd).map_err(|e|e.to_string());
    while let Ok(command)=rx.recv(){match command{
        Command::Ping{reply}=>{let _=reply.send(state.as_ref().map(|_|()).map_err(|e|(*e).clone()));}
        Command::Roots{reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(Worker::roots));}
        Command::Inspect{identity,mask,reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(|s|s.inspect(&identity,mask)));}
        Command::Children{identity,range,reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(|s|s.children(&identity,range)));}
        Command::Perform{identity,request,reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(|s|s.perform(&identity,request)));}
        Command::Text{identity,query,reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(|s|s.text(&identity,query)));}
    }}
}

impl Worker {
    fn new(hwnd:isize)->Result<Self,String>{
        let automation=UIAutomation::new().map_err(|e|format!("UIA initialization failed: {e}"))?;
        let cache=automation.create_cache_request().map_err(|e|e.to_string())?;
        cache.set_tree_scope(TreeScope::Element).map_err(|e|e.to_string())?;
        for property in [UIProperty::RuntimeId,UIProperty::BoundingRectangle,UIProperty::ProcessId,UIProperty::ControlType,UIProperty::LocalizedControlType,UIProperty::Name,UIProperty::HasKeyboardFocus,UIProperty::IsKeyboardFocusable,UIProperty::IsEnabled,UIProperty::AutomationId,UIProperty::ClassName,UIProperty::HelpText,UIProperty::IsPassword,UIProperty::IsOffscreen,UIProperty::FrameworkId,UIProperty::IsRequiredForForm,UIProperty::ItemStatus]{let _=cache.add_property(property);}
        for p in [UIPatternType::Invoke,UIPatternType::Selection,UIPatternType::Value,UIPatternType::RangeValue,UIPatternType::Scroll,UIPatternType::ExpandCollapse,UIPatternType::Grid,UIPatternType::GridItem,UIPatternType::Window,UIPatternType::SelectionItem,UIPatternType::Table,UIPatternType::TableItem,UIPatternType::Text,UIPatternType::Toggle,UIPatternType::Transform,UIPatternType::ScrollItem,UIPatternType::VirtualizedItem]{let _=cache.add_pattern(p);}
        let root=automation.element_from_handle_build_cache(Handle::from(hwnd),&cache).map_err(|e|format!("UIA root lookup failed: {e}"))?;
        let walker=automation.get_control_view_walker().map_err(|e|e.to_string())?;
        let mut worker=Self{_automation:automation,walker,cache,root:root.clone(),elements:BTreeMap::new()};worker.register(root)?;Ok(worker)
    }

    fn roots(&mut self)->Result<Vec<BackendRoot>,String>{let identity=self.register(self.root.clone())?;Ok(vec![BackendRoot{native:identity,name:self.root.get_name().ok().filter(|s|!s.is_empty()),pid:self.root.get_process_id().ok().map(|v|v as u32)}])}

    fn register(&mut self,element:UIElement)->Result<NativeIdentity,String>{
        let process_id=element.get_process_id().ok().map(|v|v as u32);let mut runtime_id=element.get_runtime_id().unwrap_or_default();
        if runtime_id.is_empty(){let fallback=format!("{}|{}|{:?}|{:?}",element.get_automation_id().unwrap_or_default(),element.get_name().unwrap_or_default(),element.get_control_type().ok(),element.get_bounding_rectangle().ok().map(|r|(r.get_left(),r.get_top(),r.get_width(),r.get_height())));runtime_id=vec![-1,stable_hash32(&fallback) as i32];}
        let identity=NativeIdentity::Uia{runtime_id,process_id};self.elements.insert(identity_key(&identity),element);Ok(identity)
    }
    fn element(&self,identity:&NativeIdentity)->Result<UIElement,String>{self.elements.get(&identity_key(identity)).cloned().ok_or_else(||"UIA element reference is stale; refresh the session".to_string())}

    fn inspect(&mut self,identity:&NativeIdentity,_mask:InspectMask)->Result<ElementSnapshot,String>{
        let element=self.element(identity)?;let control=element.get_control_type().map(|v|format!("{:?}",v)).unwrap_or_else(|_|"Unknown".into());let role=role_from_uia(&control);
        let name=element.get_name().ok().filter(|s|!s.is_empty());let automation_id=element.get_automation_id().ok().filter(|s|!s.is_empty());let mut states=StateSet::default();
        if let Ok(v)=element.is_enabled(){states.set_bool(State::Enabled,v)} if let Ok(v)=element.has_keyboard_focus(){states.set_bool(State::Focused,v)} if let Ok(v)=element.is_keyboard_focusable(){states.set_bool(State::Focusable,v)} if let Ok(v)=element.is_offscreen(){states.set_bool(State::Offscreen,v);states.set_bool(State::Visible,!v)} if let Ok(v)=element.is_required_for_form(){states.set_bool(State::Required,v)} if let Ok(v)=element.is_data_valid_for_form(){states.set_bool(State::Invalid,!v)} if let Ok(v)=element.is_password(){states.set_bool(State::Protected,v)}
        let mut capabilities=BTreeSet::new();let mut actions=Vec::new();let mut value=None;
        if cached_pattern!(&element,UIInvokePattern).is_some(){capabilities.insert(CapabilityKind::Action);actions.push(native_action("Invoke",SemanticAction::Activate));}
        if let Some(p)=cached_pattern!(&element,UIValuePattern){capabilities.insert(CapabilityKind::Value);value=p.get_value().ok().map(PropertyValue::String);if let Ok(v)=p.is_readonly(){states.set_bool(State::ReadOnly,v);states.set_bool(State::Editable,!v);}actions.push(native_action("Value",SemanticAction::SetValue));}
        if let Some(p)=cached_pattern!(&element,UIRangeValuePattern){capabilities.insert(CapabilityKind::RangeValue);if value.is_none(){value=p.get_value().ok().map(PropertyValue::F64);}if let Ok(v)=p.is_readonly(){states.set_bool(State::ReadOnly,v);states.set_bool(State::Editable,!v);}actions.push(native_action("RangeValue",SemanticAction::SetValue));actions.push(native_action("RangeValue",SemanticAction::Increment));actions.push(native_action("RangeValue",SemanticAction::Decrement));}
        if let Some(p)=cached_pattern!(&element,UISelectionItemPattern){capabilities.insert(CapabilityKind::SelectionItem);if let Ok(v)=p.is_selected(){states.set_bool(State::Selectable,true);states.set_bool(State::Selected,v);}actions.push(native_action("SelectionItem",SemanticAction::Select));}
        add_capability!(&element,UISelectionPattern,CapabilityKind::Selection,capabilities);
        if let Some(p)=cached_pattern!(&element,UIExpandCollapsePattern){capabilities.insert(CapabilityKind::ExpandCollapse);if let Ok(v)=p.get_state(){match v{ExpandCollapseState::Expanded=>{states.set_bool(State::Expandable,true);states.set_bool(State::Expanded,true)},ExpandCollapseState::Collapsed|ExpandCollapseState::PartiallyExpanded=>{states.set_bool(State::Expandable,true);states.set_bool(State::Expanded,false)},ExpandCollapseState::LeafNode=>states.set_bool(State::Expandable,false)}}actions.push(native_action("ExpandCollapse",SemanticAction::Expand));actions.push(native_action("ExpandCollapse",SemanticAction::Collapse));}
        if let Some(p)=cached_pattern!(&element,UITogglePattern){capabilities.insert(CapabilityKind::Toggle);if let Ok(v)=p.get_toggle_state(){states.set_bool(State::Checkable,true);match v{ToggleState::On=>states.set_bool(State::Checked,true),ToggleState::Off=>states.set_bool(State::Checked,false),ToggleState::Indeterminate=>states.set_bool(State::Indeterminate,true)}}actions.push(native_action("Toggle",SemanticAction::Toggle));}
        add_capability!(&element,UIScrollPattern,CapabilityKind::Scroll,capabilities);if cached_pattern!(&element,UIScrollItemPattern).is_some(){capabilities.insert(CapabilityKind::ScrollItem);actions.push(native_action("ScrollItem",SemanticAction::ScrollIntoView));}
        add_capability!(&element,UITextPattern,CapabilityKind::Text,capabilities);add_capability!(&element,UIGridPattern,CapabilityKind::Table,capabilities);add_capability!(&element,UITablePattern,CapabilityKind::Table,capabilities);add_capability!(&element,UIGridItemPattern,CapabilityKind::TableCell,capabilities);add_capability!(&element,UITableItemPattern,CapabilityKind::TableCell,capabilities);add_capability!(&element,UIWindowPattern,CapabilityKind::Window,capabilities);add_capability!(&element,UITransformPattern,CapabilityKind::Transform,capabilities);add_capability!(&element,UIVirtualizedItemPattern,CapabilityKind::VirtualizedItem,capabilities);
        let geometry=element.get_bounding_rectangle().ok().map(|r|Geometry{bounds:Rect{x:r.get_left() as f64,y:r.get_top() as f64,width:r.get_width() as f64,height:r.get_height() as f64},coordinate_space:CoordinateSpace::ScreenPhysical,visibility:if states.is_true(State::Offscreen){Visibility::Offscreen}else{Visibility::Visible},z_order:None});
        let mut properties=BTreeMap::new();if let Ok(v)=element.get_classname(){if !v.is_empty(){properties.insert("class".into(),PropertyValue::String(v));}}if let Ok(v)=element.get_localized_control_type(){if !v.is_empty(){properties.insert("localized-type".into(),PropertyValue::String(v));}}if let Ok(v)=element.get_framework_id(){if !v.is_empty(){properties.insert("framework".into(),PropertyValue::String(v));}}if let Ok(v)=element.get_item_status(){if !v.is_empty(){properties.insert("item-status".into(),PropertyValue::String(v));}}
        let description=element.get_help_text().ok().filter(|s|!s.is_empty());let native_actions=actions.iter().map(|a|a.name.clone()).collect::<Vec<_>>();let native_attributes=properties.clone();
        Ok(ElementSnapshot{node:0,identity:ElementIdentity{native:identity.clone(),stable_id:automation_id},role,native_role:NativeRole{backend:BackendKind::WindowsUia,role:control,subrole:None,numeric_id:None},name,description,value,states,capabilities,actions,geometry,parent:None,children:Vec::new(),relations:Vec::new(),properties,native:NativeMetadata{attributes:native_attributes,interfaces:BTreeSet::new(),actions:native_actions}})
    }

    fn children(&mut self,identity:&NativeIdentity,range:ChildRange)->Result<Vec<NativeIdentity>,String>{let element=self.element(identity)?;let children=self.walker.get_children_build_cache(&element,&self.cache).unwrap_or_default();let start=range.offset.min(children.len());let end=(start+range.limit).min(children.len());let mut out=Vec::new();for child in children[start..end].iter().cloned(){out.push(self.register(child)?);}Ok(out)}

    fn perform(&mut self,identity:&NativeIdentity,request:ActionRequest)->Result<BackendActionResult,String>{
        let element=self.element(identity)?;let result:Result<(),String>=match request.action{
            SemanticAction::Activate|SemanticAction::Confirm|SemanticAction::Cancel|SemanticAction::ShowMenu=>cached_pattern!(&element,UIInvokePattern).ok_or_else(||"Invoke pattern is unavailable".to_string()).and_then(|p|p.invoke().map_err(|e|e.to_string())),
            SemanticAction::Focus=>element.set_focus().map_err(|e|e.to_string()),
            SemanticAction::SetValue|SemanticAction::SetText|SemanticAction::ReplaceText=>match request.value{ActionValue::Text(v)=>cached_pattern!(&element,UIValuePattern).ok_or_else(||"Value pattern is unavailable".to_string()).and_then(|p|p.set_value(&v).map_err(|e|e.to_string())),ActionValue::Number(v)=>cached_pattern!(&element,UIRangeValuePattern).ok_or_else(||"RangeValue pattern is unavailable".to_string()).and_then(|p|p.set_value(v).map_err(|e|e.to_string())),_=>Err("set-value requires text or number".into())},
            SemanticAction::Toggle=>cached_pattern!(&element,UITogglePattern).ok_or_else(||"Toggle pattern is unavailable".to_string()).and_then(|p|p.toggle().map_err(|e|e.to_string())),SemanticAction::Select=>cached_pattern!(&element,UISelectionItemPattern).ok_or_else(||"SelectionItem pattern is unavailable".to_string()).and_then(|p|p.select().map_err(|e|e.to_string())),SemanticAction::Expand=>cached_pattern!(&element,UIExpandCollapsePattern).ok_or_else(||"ExpandCollapse pattern is unavailable".to_string()).and_then(|p|p.expand().map_err(|e|e.to_string())),SemanticAction::Collapse=>cached_pattern!(&element,UIExpandCollapsePattern).ok_or_else(||"ExpandCollapse pattern is unavailable".to_string()).and_then(|p|p.collapse().map_err(|e|e.to_string())),
            SemanticAction::Increment|SemanticAction::Decrement=>cached_pattern!(&element,UIRangeValuePattern).ok_or_else(||"RangeValue pattern is unavailable".to_string()).and_then(|p|{let current=p.get_value().map_err(|e|e.to_string())?;let next=if request.action==SemanticAction::Increment{current+1.0}else{current-1.0};p.set_value(next).map_err(|e|e.to_string())}),SemanticAction::ScrollIntoView=>cached_pattern!(&element,UIScrollItemPattern).ok_or_else(||"ScrollItem pattern is unavailable".to_string()).and_then(|p|p.scroll_into_view().map_err(|e|e.to_string())),SemanticAction::RaiseWindow=>element.set_focus().map_err(|e|e.to_string()),};
        match result{Ok(())=>Ok(BackendActionResult{success:true,native_action:Some(format!("{:?}",request.action)),message:None}),Err(e)=>Ok(BackendActionResult{success:false,native_action:None,message:Some(e)})}
    }

    fn text(&mut self,identity:&NativeIdentity,query:TextQuery)->Result<TextResult,String>{let element=self.element(identity)?;let p=cached_pattern!(&element,UITextPattern).ok_or_else(||"Text pattern is unavailable".to_string())?;let range=p.get_document_range().map_err(|e|e.to_string())?;let text=range.get_text(-1).map_err(|e|e.to_string())?;match query{TextQuery::All=>Ok(TextResult::Text(text)),TextQuery::Range(r)=>Ok(TextResult::Text(text.chars().skip(r.start).take(r.end.saturating_sub(r.start)).collect())),TextQuery::Character(i)=>Ok(TextResult::Text(text.chars().nth(i).map(|c|c.to_string()).unwrap_or_default())),TextQuery::Line(line)=>Ok(TextResult::Text(text.lines().nth(line).unwrap_or("").to_string())),TextQuery::Bounds(_)|TextQuery::Attributes(_)=>Err("range bounds/attributes are not yet exposed by the UIA actor".into())}}
}

fn native_action(name:&str,semantic:SemanticAction)->ElementAction{ElementAction{semantic:Some(semantic),native:NativeAction{backend:BackendKind::WindowsUia,name:name.into(),index:None},name:name.into(),description:None,key_binding:None}}
fn identity_key(identity:&NativeIdentity)->String{match identity{NativeIdentity::Uia{runtime_id,process_id}=>format!("{:?}:{:?}",process_id,runtime_id),other=>format!("{:?}",other)}}
fn stable_hash32(input:&str)->u32{let mut h=2166136261u32;for b in input.bytes(){h^=b as u32;h=h.wrapping_mul(16777619);}h}
fn parse_hwnd_value(value:&str)->Result<isize,String>{let raw=value.trim().trim_start_matches("hwnd:");if let Some(hex)=raw.strip_prefix("0x"){isize::from_str_radix(hex,16).map_err(|e|format!("invalid HWND: {e}"))}else{raw.parse::<isize>().map_err(|e|format!("invalid HWND: {e}"))}}
