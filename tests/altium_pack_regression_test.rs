use desktop_cli::packs::{builtin_altium, project};
use desktop_cli::semantic::*;
use std::collections::{BTreeMap, BTreeSet};

fn element(id: u64, role: Role, name: &str, parent: Option<u64>) -> ElementSnapshot {
    ElementSnapshot {
        node: id,
        identity: ElementIdentity {
            native: NativeIdentity::Synthetic {
                key: format!("altium-fixture-{id}"),
            },
            stable_id: None,
        },
        role,
        native_role: NativeRole {
            backend: BackendKind::Mock,
            role: role.as_str().into(),
            subrole: None,
            numeric_id: None,
        },
        name: Some(name.into()),
        description: None,
        value: None,
        states: StateSet::default(),
        capabilities: BTreeSet::new(),
        actions: Vec::new(),
        geometry: None,
        parent,
        children: Vec::new(),
        relations: Vec::new(),
        properties: BTreeMap::new(),
        native: NativeMetadata::default(),
    }
}

fn pcb_graph() -> AccessibilityGraph {
    let mut graph = AccessibilityGraph::new(SessionId::new());
    let mut window = element(1, Role::Window, "Altium Designer", None);
    window.children = vec![2, 3, 4, 5, 20];
    graph.insert(window);

    let mut projects = element(2, Role::Tree, "Projects", Some(1));
    projects.children = (6..=12).collect();
    graph.insert(projects);
    for id in 6..=12 {
        graph.insert(element(
            id,
            Role::TreeItem,
            if id == 6 { "Board1.PcbDoc" } else { "Library.IntLib" },
            Some(2),
        ));
    }

    let mut properties = element(3, Role::Panel, "Properties", Some(1));
    properties.children = vec![13, 14];
    graph.insert(properties);
    graph.insert(element(13, Role::TextInput, "Width", Some(3)));
    graph.insert(element(14, Role::TextInput, "Height", Some(3)));

    let mut document = element(4, Role::Document, "Board1.PcbDoc", Some(1));
    document.states.set_bool(State::Selected, true);
    graph.insert(document);

    let mut toolbar = element(5, Role::Toolbar, "PCB Toolbar", Some(1));
    toolbar.children = (15..=19).collect();
    graph.insert(toolbar);
    for id in 15..=19 {
        graph.insert(element(id, Role::Button, &format!("Tool {id}"), Some(5)));
    }

    // Deliberate structural noise: it should not survive a whitelist projection.
    let mut noise = element(20, Role::Panel, "", Some(1));
    noise.children = (21..=40).collect();
    graph.insert(noise);
    for id in 21..=40 {
        graph.insert(element(id, Role::StaticText, "decorative", Some(20)));
    }
    graph
}

#[test]
fn pcb_view_exposes_stable_altium_interface_and_compresses_noise() {
    let mut graph = pcb_graph();
    let pack = builtin_altium().unwrap();
    let source = graph.len();
    let result = project(&mut graph, &pack, ObservationBudget::default()).unwrap();

    assert_eq!(result.active_view, "pcb");
    assert!(result.aliases.contains_key("projects"));
    assert!(result.aliases.contains_key("properties"));
    assert!(result.aliases.contains_key("pcb"));
    assert!(result.observation.stats.exposed_nodes < source / 2);
    assert!(result.observation.stats.collapsed_nodes >= 1);
}

#[test]
fn modal_view_overrides_editor_view_and_never_hides_dialog() {
    let mut graph = pcb_graph();
    let mut dialog = element(50, Role::Dialog, "Confirm Delete", Some(1));
    dialog.states.set_bool(State::Modal, true);
    dialog.children = vec![51, 52];
    graph.insert(dialog);
    graph.insert(element(51, Role::Button, "OK", Some(50)));
    graph.insert(element(52, Role::Button, "Cancel", Some(50)));
    graph.get_mut(1).unwrap().children.push(50);

    let result = project(&mut graph, &builtin_altium().unwrap(), ObservationBudget::default()).unwrap();
    assert_eq!(result.active_view, "modal");
    assert!(result
        .observation
        .roots
        .iter()
        .any(|root| root.role == Role::Dialog && root.name.as_deref() == Some("Confirm Delete")));
}

#[test]
fn ambiguous_save_target_is_diagnostic_not_arbitrary_binding() {
    let mut graph = pcb_graph();
    for id in [60, 61] {
        let mut save = element(id, Role::Button, "Save", Some(1));
        save.capabilities.insert(CapabilityKind::Action);
        graph.insert(save);
        graph.get_mut(1).unwrap().children.push(id);
    }
    let result = project(&mut graph, &builtin_altium().unwrap(), ObservationBudget::default()).unwrap();
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.message.contains("save") && d.message.contains("ambiguous")));
    assert!(!result.aliases.contains_key("save"));
}

#[test]
fn sensitive_values_are_redacted_before_projection_output() {
    let mut graph = AccessibilityGraph::new(SessionId::new());
    let mut password = element(1, Role::PasswordInput, "Password", None);
    password.value = Some(PropertyValue::String("top-secret".into()));
    password.states.set_bool(State::Protected, true);
    graph.insert(password);
    let observation = observe_raw(&graph, ObservationBudget::default());
    assert_ne!(observation.roots[0].value.as_deref(), Some("top-secret"));
    assert_eq!(observation.roots[0].value.as_deref(), Some("[redacted]"));
}
