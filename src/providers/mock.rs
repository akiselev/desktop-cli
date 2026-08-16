use crate::semantic::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{self, Receiver, Sender};

pub struct MockAccessibilityBackend {
    nodes: BTreeMap<String, ElementSnapshot>,
    node_keys: BTreeMap<NodeId, String>,
    roots: Vec<String>,
    subscribers: Vec<Sender<AccessibilityEvent>>,
}

impl MockAccessibilityBackend {
    pub fn new(nodes: Vec<ElementSnapshot>, roots: Vec<String>) -> Self {
        let mut map = BTreeMap::new(); let mut node_keys = BTreeMap::new();
        for node in nodes { let key = key_for_identity(&node.identity.native); node_keys.insert(node.node, key.clone()); map.insert(key, node); }
        Self { nodes: map, node_keys, roots, subscribers: Vec::new() }
    }

    pub fn simple_form() -> Self {
        let mut root = mock_element(1, Role::Window, "Example", None); root.children = vec![2,3,4];
        let mut input = mock_element(2, Role::TextInput, "Name", Some(1)); input.capabilities.insert(CapabilityKind::Value); input.value = Some(PropertyValue::String(String::new()));
        let mut toggle = mock_element(3, Role::Checkbox, "Enabled", Some(1)); toggle.capabilities.insert(CapabilityKind::Toggle); toggle.states.set_bool(State::Checked, false);
        let mut button = mock_element(4, Role::Button, "Save", Some(1)); button.capabilities.insert(CapabilityKind::Action); button.identity.stable_id = Some("save".into());
        Self::new(vec![root,input,toggle,button],vec!["mock:1".into()])
    }

    fn emit(&mut self, event: AccessibilityEvent) { self.subscribers.retain(|tx| tx.send(event.clone()).is_ok()); }
    fn node_mut(&mut self, identity: &NativeIdentity) -> Result<&mut ElementSnapshot,String> { let key=key_for_identity(identity); self.nodes.get_mut(&key).ok_or_else(||format!("missing mock node {}",key)) }
}

impl AccessibilityBackend for MockAccessibilityBackend {
    fn kind(&self) -> BackendKind { BackendKind::Mock }
    fn roots(&mut self) -> Result<Vec<BackendRoot>,String> { Ok(self.roots.iter().filter_map(|key|self.nodes.get(key).map(|n|BackendRoot{native:n.identity.native.clone(),name:n.name.clone(),pid:None})).collect()) }
    fn inspect(&mut self, identity:&NativeIdentity,_mask:InspectMask)->Result<ElementSnapshot,String>{self.nodes.get(&key_for_identity(identity)).cloned().ok_or_else(||"mock element not found".into())}
    fn children(&mut self, identity:&NativeIdentity, range:ChildRange)->Result<Vec<NativeIdentity>,String>{let node=self.nodes.get(&key_for_identity(identity)).ok_or_else(||"mock element not found".to_string())?;let start=range.offset.min(node.children.len());let end=(start+range.limit).min(node.children.len());Ok(node.children[start..end].iter().filter_map(|id|self.node_keys.get(id)).filter_map(|k|self.nodes.get(k)).map(|n|n.identity.native.clone()).collect())}
    fn perform(&mut self, identity:&NativeIdentity, request:ActionRequest)->Result<BackendActionResult,String>{
        let mut event=None;
        {
            let node=self.node_mut(identity)?;
            match request.action {
                SemanticAction::SetValue|SemanticAction::SetText|SemanticAction::ReplaceText=>{let text=match request.value{ActionValue::Text(v)=>v,ActionValue::Number(v)=>v.to_string(),ActionValue::Bool(v)=>v.to_string(),ActionValue::Range(v)=>format!("{}..{}",v.start,v.end),ActionValue::None=>String::new()};node.value=Some(PropertyValue::String(text));event=Some(AccessibilityEvent::PropertyChanged{element:node.identity.native.clone(),property:"value".into()});}
                SemanticAction::Toggle=>{let next=!node.states.is_true(State::Checked);node.states.set_bool(State::Checked,next);event=Some(AccessibilityEvent::StateChanged{element:node.identity.native.clone(),state:State::Checked,value:next});}
                SemanticAction::Select=>{node.states.set_bool(State::Selected,true);event=Some(AccessibilityEvent::StateChanged{element:node.identity.native.clone(),state:State::Selected,value:true});}
                SemanticAction::Expand=>{node.states.set_bool(State::Expanded,true);event=Some(AccessibilityEvent::StateChanged{element:node.identity.native.clone(),state:State::Expanded,value:true});}
                SemanticAction::Collapse=>{node.states.set_bool(State::Expanded,false);event=Some(AccessibilityEvent::StateChanged{element:node.identity.native.clone(),state:State::Expanded,value:false});}
                SemanticAction::Focus=>{node.states.set_bool(State::Focused,true);event=Some(AccessibilityEvent::FocusChanged{old:None,new:Some(node.identity.native.clone())});}
                SemanticAction::Increment|SemanticAction::Decrement=>{let current=node.value.as_ref().and_then(|v|match v{PropertyValue::F64(v)=>Some(*v),PropertyValue::I64(v)=>Some(*v as f64),_=>None}).unwrap_or(0.0);let next=if request.action==SemanticAction::Increment{current+1.0}else{current-1.0};node.value=Some(PropertyValue::F64(next));event=Some(AccessibilityEvent::PropertyChanged{element:node.identity.native.clone(),property:"value".into()});}
                _=>{}
            }
        }
        if let Some(event)=event{self.emit(event);}
        Ok(BackendActionResult{success:true,native_action:Some(format!("mock:{:?}",request.action)),message:None})
    }
    fn subscribe(&mut self,_root:&NativeIdentity,_interests:&BTreeSet<EventKind>)->Result<Receiver<AccessibilityEvent>,String>{let(tx,rx)=mpsc::channel();self.subscribers.push(tx);Ok(rx)}
}

fn key_for_identity(identity:&NativeIdentity)->String{match identity{NativeIdentity::Synthetic{key}=>key.clone(),other=>format!("{:?}",other)}}
fn mock_element(id:NodeId,role:Role,name:&str,parent:Option<NodeId>)->ElementSnapshot{ElementSnapshot{node:id,identity:ElementIdentity{native:NativeIdentity::Synthetic{key:format!("mock:{id}")},stable_id:None},role,native_role:NativeRole{backend:BackendKind::Mock,role:role.as_str().into(),subrole:None,numeric_id:None},name:Some(name.into()),description:None,value:None,states:StateSet::default(),capabilities:BTreeSet::new(),actions:Vec::new(),geometry:Some(Geometry{bounds:Rect{x:0.0,y:(id*30)as f64,width:200.0,height:24.0},coordinate_space:CoordinateSpace::ScreenLogical,visibility:Visibility::Visible,z_order:None}),parent,children:Vec::new(),relations:Vec::new(),properties:BTreeMap::new(),native:NativeMetadata::default()}}
