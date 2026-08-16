use desktop_cli::providers::MockAccessibilityBackend;
use desktop_cli::semantic::{ObservationBudget, SemanticAction};
use desktop_cli::session::protocol::{handle_request, SessionRequest};
use desktop_cli::session::DesktopSession;

#[test]
fn hello_negotiates_protocol_and_backend() {
    let mut session = DesktopSession::new(Box::new(MockAccessibilityBackend::simple_form()), None);
    session.refresh().unwrap();
    let response = handle_request(&mut session, SessionRequest::Hello);
    assert!(response.ok);
    let value = response.result.unwrap();
    assert_eq!(value["protocol"], 1);
    assert!(value.get("session").is_some());
    assert_eq!(value["backend"], "mock");
}

#[test]
fn observe_is_bounded_and_versioned() {
    let mut session = DesktopSession::new(Box::new(MockAccessibilityBackend::simple_form()), None);
    session.refresh().unwrap();
    let response = handle_request(
        &mut session,
        SessionRequest::Observe {
            budget: Some(ObservationBudget {
                max_nodes: 2,
                ..ObservationBudget::default()
            }),
        },
    );
    assert!(response.ok);
    let value = response.result.unwrap();
    assert_eq!(value["observation"]["schema_version"], 2);
    assert!(value["observation"]["stats"]["exposed_nodes"].as_u64().unwrap() <= 2);
}

#[test]
fn action_response_uses_portable_action_vocabulary() {
    let mut session = DesktopSession::new(Box::new(MockAccessibilityBackend::simple_form()), None);
    session.refresh().unwrap();
    let target = session.query("@checkbox").unwrap()[0].opaque.clone();
    let response = handle_request(
        &mut session,
        SessionRequest::Action {
            target,
            action: SemanticAction::Toggle.as_str().to_string(),
            value: None,
            allow_input_fallback: Some(false),
        },
    );
    assert!(response.ok);
    let value = response.result.unwrap();
    assert_eq!(value["schema_version"], 2);
    assert_eq!(value["action"], "toggle");
    assert_eq!(value["route"], "accessibility");
    assert!(value.get("observation").is_some());
}

#[test]
fn malformed_action_is_a_protocol_error_not_a_guess() {
    let mut session = DesktopSession::new(Box::new(MockAccessibilityBackend::simple_form()), None);
    session.refresh().unwrap();
    let response = handle_request(
        &mut session,
        SessionRequest::Action {
            target: "e_invalid".into(),
            action: "please-do-the-thing".into(),
            value: None,
            allow_input_fallback: None,
        },
    );
    assert!(!response.ok);
    assert!(response.error.unwrap().contains("unknown semantic action"));
}
