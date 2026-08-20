#![cfg(target_os = "linux")]

use crate::providers::linux::{capability_from_interface, relation_from_atspi, role_from_atspi, semantic_action_from_name};
use crate::semantic::*;
use atspi::proxy::accessible::AccessibleProxy;
use atspi::proxy::action::ActionProxy;
use atspi::proxy::component::ComponentProxy;
use atspi::proxy::text::TextProxy;
use atspi::proxy::value::ValueProxy;
use atspi::{AccessibilityConnection, CoordType, Interface, InterfaceSet};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use tokio::runtime::Runtime;
use zbus::fdo::DBusProxy;

pub struct LinuxAtSpiBackend { tx: Sender<Command> }

impl LinuxAtSpiBackend {
    pub fn new(pid: u32, title: impl Into<String>) -> Result<Self, String> {
        let title = title.into();
        let (tx, rx) = mpsc::channel();
        thread::Builder::new().name("desktop-cli-atspi".into()).spawn(move || worker_main(pid, title, rx)).map_err(|e| format!("failed to spawn AT-SPI actor: {e}"))?;
        let backend = Self { tx };
        backend.call(|reply| Command::Ping { reply })?;
        Ok(backend)
    }

    fn call<T: Send + 'static>(&self, make: impl FnOnce(Sender<Result<T, String>>) -> Command) -> Result<T, String> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(make(tx)).map_err(|_| "AT-SPI actor disconnected".to_string())?;
        rx.recv().map_err(|_| "AT-SPI actor disconnected".to_string())?
    }
}

impl AccessibilityBackend for LinuxAtSpiBackend {
    fn kind(&self) -> BackendKind { BackendKind::LinuxAtSpi }
    fn roots(&mut self) -> Result<Vec<BackendRoot>, String> { self.call(|reply| Command::Roots { reply }) }
    fn inspect(&mut self, identity: &NativeIdentity, mask: InspectMask) -> Result<ElementSnapshot, String> { let identity=identity.clone(); self.call(|reply| Command::Inspect { identity, mask, reply }) }
    fn children(&mut self, identity: &NativeIdentity, range: ChildRange) -> Result<Vec<NativeIdentity>, String> { let identity=identity.clone(); self.call(|reply| Command::Children { identity, range, reply }) }
    fn perform(&mut self, identity: &NativeIdentity, request: ActionRequest) -> Result<BackendActionResult, String> { let identity=identity.clone(); self.call(|reply| Command::Perform { identity, request, reply }) }
    fn query_text(&mut self, identity: &NativeIdentity, query: TextQuery) -> Result<TextResult, String> { let identity=identity.clone(); self.call(|reply| Command::Text { identity, query, reply }) }

    fn plan_query(&self, selector: &Selector) -> NativeQueryPlan {
        let mut pushed=Vec::new();let mut residual=Vec::new();
        for step in &selector.steps { for test in &step.tests { match test {
            SelectorTest::Role(role)=>pushed.push(format!("role={}",role.as_str())),
            SelectorTest::State(p)=>pushed.push(format!("state.{:?}={:?}",p.state,p.expected)),
            SelectorTest::Capability(cap)=>pushed.push(format!("interface={:?}",cap)),
            SelectorTest::Property(p) if p.name.starts_with("native.")=>pushed.push(format!("{} {:?} {}",p.name,p.op,p.value)),
            other=>residual.push(format!("{:?}",other)),
        }}}
        NativeQueryPlan { backend:BackendKind::LinuxAtSpi, description:"AT-SPI Collection-compatible role/state/interface/attribute predicates are identified for pushdown; unsupported structural/relation/spatial predicates remain semantic residuals".into(), pushed_predicates:pushed, residual_selector:if residual.is_empty(){String::new()}else{selector.source.clone()}, exact:residual.is_empty() }
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

struct Worker { rt:Runtime, connection:AccessibilityConnection, root:NativeIdentity, pid:u32 }

fn worker_main(pid:u32,title:String,rx:Receiver<Command>){
    let mut state=Worker::new(pid,&title);
    while let Ok(command)=rx.recv(){match command{
        Command::Ping{reply}=>{let _=reply.send(state.as_ref().map(|_|()).map_err(Clone::clone));}
        Command::Roots{reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(Worker::roots));}
        Command::Inspect{identity,mask,reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(|s|s.inspect(&identity,mask)));}
        Command::Children{identity,range,reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(|s|s.children(&identity,range)));}
        Command::Perform{identity,request,reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(|s|s.perform(&identity,request)));}
        Command::Text{identity,query,reply}=>{let _=reply.send(state.as_mut().map_err(|e|(*e).clone()).and_then(|s|s.text(&identity,query)));}
    }}
}

impl Worker {
    fn new(pid:u32,title:&str)->Result<Self,String>{
        let rt=Runtime::new().map_err(|e|e.to_string())?;
        let connection=rt.block_on(AccessibilityConnection::new()).map_err(|e|format!("failed to connect to AT-SPI: {e}"))?;
        let root=rt.block_on(find_target_root(&connection,pid,title))?;
        Ok(Self{rt,connection,root,pid})
    }

    fn roots(&mut self)->Result<Vec<BackendRoot>,String>{
        let root=self.root.clone();let conn=self.connection.connection().clone();let pid=self.pid;
        self.rt.block_on(async move { let p=accessible_proxy(&conn,&root).await?;let name=p.name().await.ok().filter(|s|!s.is_empty());Ok(vec![BackendRoot{native:root,name,pid:Some(pid)}]) })
    }

    fn inspect(&mut self,identity:&NativeIdentity,_mask:InspectMask)->Result<ElementSnapshot,String>{let identity=identity.clone();let conn=self.connection.connection().clone();self.rt.block_on(async move{inspect_async(&conn,&identity).await})}
    fn children(&mut self,identity:&NativeIdentity,range:ChildRange)->Result<Vec<NativeIdentity>,String>{let identity=identity.clone();let conn=self.connection.connection().clone();self.rt.block_on(async move{let p=accessible_proxy(&conn,&identity).await?;let children=p.get_children().await.map_err(|e|e.to_string())?;let start=range.offset.min(children.len());let end=(start+range.limit).min(children.len());Ok(children[start..end].iter().map(object_ref_identity).collect())})}
    fn perform(&mut self,identity:&NativeIdentity,request:ActionRequest)->Result<BackendActionResult,String>{let identity=identity.clone();let conn=self.connection.connection().clone();self.rt.block_on(async move{perform_async(&conn,&identity,request).await})}
    fn text(&mut self,identity:&NativeIdentity,query:TextQuery)->Result<TextResult,String>{let identity=identity.clone();let conn=self.connection.connection().clone();self.rt.block_on(async move{text_async(&conn,&identity,query).await})}
}

async fn find_target_root(connection:&AccessibilityConnection,pid:u32,title:&str)->Result<NativeIdentity,String>{
    let conn=connection.connection();
    let dbus=DBusProxy::new(conn).await.map_err(|e|e.to_string())?;
    let registry=AccessibleProxy::builder(conn).destination("org.a11y.atspi.Registry").map_err(|e|e.to_string())?.path("/org/a11y/atspi/accessible/root").map_err(|e|e.to_string())?.build().await.map_err(|e|e.to_string())?;
    for app_ref in registry.get_children().await.map_err(|e|e.to_string())? {
        let app_pid=dbus.get_connection_unix_process_id(app_ref.name.as_str().try_into().map_err(|e|format!("invalid AT-SPI bus name: {e}"))?).await.ok();
        if app_pid!=Some(pid){continue;}
        let app_root=NativeIdentity::AtSpi{bus_name:app_ref.name.to_string(),object_path:"/org/a11y/atspi/accessible/root".into()};
        let app=accessible_proxy(conn,&app_root).await?;
        let children=app.get_children().await.unwrap_or_default();
        let title_lower=title.to_ascii_lowercase();
        for child in children {
            let identity=object_ref_identity(&child);let proxy=accessible_proxy(conn,&identity).await?;let name=proxy.name().await.unwrap_or_default();
            if !title_lower.is_empty() && (name.eq_ignore_ascii_case(title)||name.to_ascii_lowercase().contains(&title_lower)){return Ok(identity);}
        }
        return Ok(app_root);
    }
    Err(format!("no AT-SPI application found for pid {pid}"))
}

async fn inspect_async(conn:&zbus::Connection,identity:&NativeIdentity)->Result<ElementSnapshot,String>{
    let p=accessible_proxy(conn,identity).await?;
    let name=p.name().await.ok().filter(|s|!s.is_empty());
    let role_native=p.get_role().await.map(|r|format!("{:?}",r)).unwrap_or_else(|_|"Unknown".into());
    let role=role_from_atspi(&normalize_name(&role_native));
    let states_native=p.get_state().await.unwrap_or_default();let mut states=StateSet::default();
    for (native,semantic) in [(atspi::State::Enabled,State::Enabled),(atspi::State::Visible,State::Visible),(atspi::State::Showing,State::Showing),(atspi::State::Focused,State::Focused),(atspi::State::Focusable,State::Focusable),(atspi::State::Editable,State::Editable),(atspi::State::Selected,State::Selected),(atspi::State::Selectable,State::Selectable),(atspi::State::Checked,State::Checked),(atspi::State::Expanded,State::Expanded),(atspi::State::Expandable,State::Expandable),(atspi::State::Sensitive,State::Sensitive),(atspi::State::Busy,State::Busy),(atspi::State::Modal,State::Modal),(atspi::State::Required,State::Required),(atspi::State::Defunct,State::Defunct)]{states.set_bool(semantic,states_native.contains(native));}
    let interfaces=p.get_interfaces().await.unwrap_or_else(|_|InterfaceSet::empty());let mut capabilities=BTreeSet::new();let mut native_interfaces=BTreeSet::new();
    for (native,name,cap) in [(Interface::Action,"Action",CapabilityKind::Action),(Interface::Value,"Value",CapabilityKind::Value),(Interface::Text,"Text",CapabilityKind::Text),(Interface::EditableText,"EditableText",CapabilityKind::EditableText),(Interface::Selection,"Selection",CapabilityKind::Selection),(Interface::Table,"Table",CapabilityKind::Table),(Interface::TableCell,"TableCell",CapabilityKind::TableCell),(Interface::Document,"Document",CapabilityKind::Document),(Interface::Component,"Component",CapabilityKind::Component),(Interface::Hypertext,"Hypertext",CapabilityKind::Hypertext),(Interface::Image,"Image",CapabilityKind::Image)]{if interfaces.contains(native){native_interfaces.insert(name.into());capabilities.insert(cap);}}
    let mut actions=Vec::new();
    if interfaces.contains(Interface::Action){if let Ok(proxy)=action_proxy(conn,identity).await{if let Ok(native_actions)=proxy.get_actions().await{for (index,a) in native_actions.into_iter().enumerate(){let semantic=semantic_action_from_name(&a.name);actions.push(ElementAction{semantic,native:NativeAction{backend:BackendKind::LinuxAtSpi,name:a.name.clone(),index:Some(index as i32)},name:a.name,description:(!a.description.is_empty()).then_some(a.description),key_binding:(!a.key_binding.is_empty()).then_some(a.key_binding)});}}}}
    let mut value=None;if interfaces.contains(Interface::Value){if let Ok(proxy)=value_proxy(conn,identity).await{value=proxy.current_value().await.ok().map(PropertyValue::F64);}}
    if value.is_none()&&interfaces.contains(Interface::Text){if let Ok(proxy)=text_proxy(conn,identity).await{let count=proxy.character_count().await.unwrap_or(0);let max=count.min(4096);value=proxy.get_text(0,max).await.ok().filter(|s|!s.is_empty()).map(PropertyValue::String);}}
    let geometry=if interfaces.contains(Interface::Component){component_proxy(conn,identity).await.ok().and_then(|c|futures_executor_block_on_extents(c)).map(|(x,y,w,h)|Geometry{bounds:Rect{x:x as f64,y:y as f64,width:w as f64,height:h as f64},coordinate_space:CoordinateSpace::ScreenLogical,visibility:if states.is_true(State::Visible){Visibility::Visible}else{Visibility::Unknown},z_order:None})}else{None};
    let mut properties=BTreeMap::new();let attrs=p.get_attributes().await.unwrap_or_default();for(k,v)in attrs{properties.insert(format!("native.{k}"),PropertyValue::String(v));}
    let stable_id=properties.get("native.id").or_else(||properties.get("native.accessible-id")).and_then(|v|match v{PropertyValue::String(s) if !s.is_empty()=>Some(s.clone()),_=>None});
    let mut relations=Vec::new();if let Ok(set)=p.get_relation_set().await{for(kind,targets)in set{relations.push(Relation{kind:relation_from_atspi(&normalize_name(&format!("{:?}",kind))),targets:Vec::new(),native_name:Some(format!("{:?}",kind))});let _=targets;}}
    Ok(ElementSnapshot{node:0,identity:ElementIdentity{native:identity.clone(),stable_id},role,native_role:NativeRole{backend:BackendKind::LinuxAtSpi,role:role_native,subrole:None,numeric_id:None},name,description:properties.get("native.description").map(PropertyValue::display_text),value,states,capabilities,actions:actions.clone(),geometry,parent:None,children:Vec::new(),relations,properties:properties.clone(),native:NativeMetadata{attributes:properties,interfaces:native_interfaces,actions:actions.into_iter().map(|a|a.name).collect()}})
}

async fn perform_async(conn:&zbus::Connection,identity:&NativeIdentity,request:ActionRequest)->Result<BackendActionResult,String>{
    match request.action {
        SemanticAction::SetValue|SemanticAction::Increment|SemanticAction::Decrement=>{let proxy=value_proxy(conn,identity).await?;let current=proxy.current_value().await.map_err(|e|e.to_string())?;let next=match request.action{SemanticAction::Increment=>current+1.0,SemanticAction::Decrement=>current-1.0,_=>match request.value{ActionValue::Number(v)=>v,ActionValue::Text(v)=>v.parse().map_err(|_|"AT-SPI Value requires a number".to_string())?,_=>return Err("AT-SPI Value requires a number".into())}};proxy.set_current_value(next).await.map_err(|e|e.to_string())?;Ok(BackendActionResult{success:true,native_action:Some("Value.SetCurrentValue".into()),message:None})}
        _=>{let proxy=action_proxy(conn,identity).await?;let actions=proxy.get_actions().await.unwrap_or_default();let mut chosen=request.native_hint.as_ref().and_then(|n|n.index).unwrap_or(-1);if chosen<0{for(i,a)in actions.iter().enumerate(){if semantic_action_from_name(&a.name)==Some(request.action){chosen=i as i32;break;}}}if chosen<0{chosen=0;}let ok=proxy.do_action(chosen).await.map_err(|e|e.to_string())?;Ok(BackendActionResult{success:ok,native_action:Some(format!("Action[{chosen}]")),message:(!ok).then(||"AT-SPI action returned false".into())})}
    }
}

async fn text_async(conn:&zbus::Connection,identity:&NativeIdentity,query:TextQuery)->Result<TextResult,String>{let p=text_proxy(conn,identity).await?;let count=p.character_count().await.map_err(|e|e.to_string())?;match query{TextQuery::All=>Ok(TextResult::Text(p.get_text(0,count).await.map_err(|e|e.to_string())?)),TextQuery::Range(r)=>Ok(TextResult::Text(p.get_text(r.start as i32,r.end as i32).await.map_err(|e|e.to_string())?)),TextQuery::Character(i)=>Ok(TextResult::Text(p.get_text(i as i32,i.saturating_add(1) as i32).await.map_err(|e|e.to_string())?)),TextQuery::Line(_)=>Err("line queries require boundary traversal and are not exposed yet".into()),TextQuery::Bounds(_)|TextQuery::Attributes(_)=>Err("AT-SPI text range bounds/attributes are not exposed yet".into())}}

async fn accessible_proxy<'a>(conn:&'a zbus::Connection,identity:&NativeIdentity)->Result<AccessibleProxy<'a>,String>{let(bus,path)=atspi_parts(identity)?;AccessibleProxy::builder(conn).destination(bus).map_err(|e|e.to_string())?.path(path).map_err(|e|e.to_string())?.build().await.map_err(|e|e.to_string())}
async fn action_proxy<'a>(conn:&'a zbus::Connection,identity:&NativeIdentity)->Result<ActionProxy<'a>,String>{let(bus,path)=atspi_parts(identity)?;ActionProxy::builder(conn).destination(bus).map_err(|e|e.to_string())?.path(path).map_err(|e|e.to_string())?.build().await.map_err(|e|e.to_string())}
async fn value_proxy<'a>(conn:&'a zbus::Connection,identity:&NativeIdentity)->Result<ValueProxy<'a>,String>{let(bus,path)=atspi_parts(identity)?;ValueProxy::builder(conn).destination(bus).map_err(|e|e.to_string())?.path(path).map_err(|e|e.to_string())?.build().await.map_err(|e|e.to_string())}
async fn text_proxy<'a>(conn:&'a zbus::Connection,identity:&NativeIdentity)->Result<TextProxy<'a>,String>{let(bus,path)=atspi_parts(identity)?;TextProxy::builder(conn).destination(bus).map_err(|e|e.to_string())?.path(path).map_err(|e|e.to_string())?.build().await.map_err(|e|e.to_string())}
async fn component_proxy<'a>(conn:&'a zbus::Connection,identity:&NativeIdentity)->Result<ComponentProxy<'a>,String>{let(bus,path)=atspi_parts(identity)?;ComponentProxy::builder(conn).destination(bus).map_err(|e|e.to_string())?.path(path).map_err(|e|e.to_string())?.build().await.map_err(|e|e.to_string())}
fn futures_executor_block_on_extents(proxy:ComponentProxy<'_>)->Option<(i32,i32,i32,i32)>{tokio::task::block_in_place(||tokio::runtime::Handle::current().block_on(proxy.get_extents(CoordType::Screen).ok()?))}
fn atspi_parts(identity:&NativeIdentity)->Result<(&str,&str),String>{match identity{NativeIdentity::AtSpi{bus_name,object_path}=>Ok((bus_name.as_str(),object_path.as_str())),_=>Err("expected AT-SPI identity".into())}}
fn object_ref_identity(r:&atspi::ObjectRef)->NativeIdentity{NativeIdentity::AtSpi{bus_name:r.name.to_string(),object_path:r.path.to_string()}}
fn normalize_name(input:&str)->String{let mut out=String::new();for(i,c)in input.chars().enumerate(){if c.is_uppercase()&&i>0{out.push('-');}out.extend(c.to_lowercase());}out.replace('_',"-")}
