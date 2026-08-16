use desktop_cli::providers::MockAccessibilityBackend;
use desktop_cli::semantic::*;
use desktop_cli::session::DesktopSession;

#[test]
fn session_refresh_query_action_and_diff_are_backend_independent(){
    let backend=Box::new(MockAccessibilityBackend::simple_form());
    let mut session=DesktopSession::new(backend,None);
    session.refresh().unwrap();
    let matches=session.query("@input[name=\"Name\"]").unwrap();assert_eq!(matches.len(),1);
    let before=session.observe(ObservationBudget::default()).unwrap().observation;
    let result=session.perform(&matches[0].opaque,SemanticAction::SetValue,ActionValue::Text("Ada".into()),false).unwrap();
    assert!(result.success);assert_eq!(result.route,"accessibility");assert!(result.after_revision>=result.before_revision);assert!(result.observation.is_some());
    let after=session.observe(ObservationBudget::default()).unwrap().observation;assert_ne!(before,after);
    let node=session.graph().resolve_ref(&session.query("@input[name=\"Name\"]").unwrap()[0]).unwrap();assert_eq!(session.graph().get(node).unwrap().value.as_ref().unwrap().display_text(),"Ada");
}

#[test]
fn session_event_subscription_updates_graph(){
    let backend=Box::new(MockAccessibilityBackend::simple_form());let mut session=DesktopSession::new(backend,None);session.refresh().unwrap();session.start_events().unwrap();
    let toggle=session.query("@checkbox").unwrap()[0].clone();session.action_policy_mut().allow_native_actions=true;
    session.perform(&toggle.opaque,SemanticAction::Toggle,ActionValue::None,false).unwrap();
    let _=session.apply_pending_events(16).unwrap();let node=session.graph().resolve_ref(&session.query("@checkbox").unwrap()[0]).unwrap();assert!(session.graph().get(node).unwrap().states.is_true(State::Checked));
}

#[test]
fn action_policy_is_independent_of_pack_projection(){
    let backend=Box::new(MockAccessibilityBackend::simple_form());let mut session=DesktopSession::new(backend,None);session.refresh().unwrap();
    session.action_policy_mut().denied_actions.insert(ActionClass::Edit);
    let input=session.query("@input").unwrap()[0].clone();let err=session.perform(&input.opaque,SemanticAction::SetValue,ActionValue::Text("blocked".into()),false).unwrap_err();assert!(err.contains("denied"));
}

#[test]
fn observation_budgets_bound_text_depth_and_nodes(){
    let backend=Box::new(MockAccessibilityBackend::simple_form());let mut session=DesktopSession::new(backend,None);session.refresh().unwrap();
    let obs=session.observe(ObservationBudget{max_nodes:2,max_text_chars:4,max_collection_items:1,max_depth:1,max_native_property_bytes:8,max_millis:1000}).unwrap().observation;
    assert!(obs.stats.truncated);assert!(obs.stats.exposed_nodes<=2);assert!(obs.stats.emitted_text_chars<=4);
}
