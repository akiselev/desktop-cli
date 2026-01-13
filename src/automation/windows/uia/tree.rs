//! UIA element tree traversal and dumping

use crate::rpc::types::{TreeDumpOptions, UiaElement};
use super::selector::{Selector, SelectorSegment};
use uiautomation::patterns::{
    UIExpandCollapsePattern, UIGridPattern, UIInvokePattern, UIRangeValuePattern,
    UIScrollPattern, UISelectionItemPattern, UISelectionPattern, UITablePattern,
    UITextPattern, UITogglePattern, UITransformPattern, UIValuePattern, UIWindowPattern,
};
use uiautomation::types::Handle;
use uiautomation::{UIAutomation, UIElement, UITreeWalker};

/// Convert a UIElement to our serializable UiaElement
pub fn element_to_uia(elem: &UIElement, depth: u32) -> UiaElement {
    let runtime_id = elem
        .get_runtime_id()
        .map(|ids| {
            ids.iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(".")
        })
        .unwrap_or_default();

    let control_type = elem
        .get_control_type()
        .map(|ct| format!("{:?}", ct))
        .unwrap_or_default();

    let localized_type = elem.get_localized_control_type().unwrap_or_default();
    let name = elem.get_name().unwrap_or_default();
    let automation_id = elem.get_automation_id().unwrap_or_default();
    let class_name = elem.get_classname().unwrap_or_default();

    // Try to get value from ValuePattern
    let value = elem
        .get_pattern::<UIValuePattern>()
        .ok()
        .and_then(|p| p.get_value().ok());

    // Get bounding rect
    let bounds = elem
        .get_bounding_rectangle()
        .map(|r| [r.get_left(), r.get_top(), r.get_width(), r.get_height()])
        .unwrap_or([0, 0, 0, 0]);

    let is_enabled = elem.is_enabled().unwrap_or(true);
    let is_offscreen = elem.is_offscreen().unwrap_or(false);

    // Detect supported patterns
    let patterns = detect_patterns(elem);

    UiaElement {
        id: runtime_id,
        control_type,
        localized_type,
        name,
        automation_id,
        class_name,
        value,
        bounds,
        is_enabled,
        is_offscreen,
        patterns,
        depth,
        children: Vec::new(),
    }
}

/// Detect which patterns an element supports
fn detect_patterns(elem: &UIElement) -> Vec<String> {
    let mut patterns = Vec::new();

    if elem.get_pattern::<UIInvokePattern>().is_ok() {
        patterns.push("Invoke".to_string());
    }
    if elem.get_pattern::<UIValuePattern>().is_ok() {
        patterns.push("Value".to_string());
    }
    if elem.get_pattern::<UITogglePattern>().is_ok() {
        patterns.push("Toggle".to_string());
    }
    if elem.get_pattern::<UISelectionItemPattern>().is_ok() {
        patterns.push("SelectionItem".to_string());
    }
    if elem.get_pattern::<UISelectionPattern>().is_ok() {
        patterns.push("Selection".to_string());
    }
    if elem.get_pattern::<UIExpandCollapsePattern>().is_ok() {
        patterns.push("ExpandCollapse".to_string());
    }
    if elem.get_pattern::<UIScrollPattern>().is_ok() {
        patterns.push("Scroll".to_string());
    }
    if elem.get_pattern::<UITextPattern>().is_ok() {
        patterns.push("Text".to_string());
    }
    if elem.get_pattern::<UIRangeValuePattern>().is_ok() {
        patterns.push("RangeValue".to_string());
    }
    if elem.get_pattern::<UIGridPattern>().is_ok() {
        patterns.push("Grid".to_string());
    }
    if elem.get_pattern::<UITablePattern>().is_ok() {
        patterns.push("Table".to_string());
    }
    if elem.get_pattern::<UIWindowPattern>().is_ok() {
        patterns.push("Window".to_string());
    }
    if elem.get_pattern::<UITransformPattern>().is_ok() {
        patterns.push("Transform".to_string());
    }

    patterns
}

/// Dump the UIA element tree starting from a root element
pub fn dump_tree(
    automation: &UIAutomation,
    root: &UIElement,
    options: &TreeDumpOptions,
) -> Result<UiaElement, uiautomation::Error> {
    let walker = automation.create_tree_walker()?;
    dump_element_recursive(&walker, root, 0, options)
}

fn dump_element_recursive(
    walker: &UITreeWalker,
    elem: &UIElement,
    depth: u32,
    options: &TreeDumpOptions,
) -> Result<UiaElement, uiautomation::Error> {
    let mut uia_elem = element_to_uia(elem, depth);

    // Apply pruning
    if options.prune_offscreen && uia_elem.is_offscreen {
        // Still return the element but mark it, caller can filter
    }

    // Recurse into children if within depth limit
    if depth < options.max_depth {
        let mut child_count = 0;

        if let Ok(first_child) = walker.get_first_child(elem) {
            let mut current = first_child;

            loop {
                // Check list item cap
                if options.max_list_items > 0 && child_count >= options.max_list_items {
                    // Add a placeholder indicating more items
                    let mut placeholder = UiaElement::default();
                    placeholder.name = format!("... ({} more items)", "?");
                    placeholder.depth = depth + 1;
                    uia_elem.children.push(placeholder);
                    break;
                }

                let child_elem = dump_element_recursive(walker, &current, depth + 1, options)?;

                // Apply pruning rules
                let should_include = if options.prune_offscreen && child_elem.is_offscreen {
                    false
                } else if options.prune_empty
                    && child_elem.name.is_empty()
                    && child_elem.patterns.is_empty()
                    && child_elem.children.is_empty()
                {
                    false
                } else {
                    true
                };

                if should_include {
                    uia_elem.children.push(child_elem);
                    child_count += 1;
                }

                match walker.get_next_sibling(&current) {
                    Ok(sibling) => current = sibling,
                    Err(_) => break,
                }
            }
        }
    }

    Ok(uia_elem)
}

/// Get the UIA element from a window handle
pub fn element_from_hwnd(
    automation: &UIAutomation,
    hwnd: isize,
) -> Result<UIElement, uiautomation::Error> {
    automation.element_from_handle(Handle::from(hwnd))
}

/// Find elements matching a selector
pub fn find_elements(
    automation: &UIAutomation,
    root: &UIElement,
    selector: &Selector,
    find_all: bool,
    timeout_ms: u64,
) -> Result<Vec<UiaElement>, uiautomation::Error> {
    let mut results = Vec::new();

    // Start with the first segment
    if selector.segments.is_empty() {
        return Ok(results);
    }

    // Use UIA's built-in search for the first segment when possible
    let first_seg = &selector.segments[0];
    let mut candidates = find_by_segment(automation, root, first_seg, timeout_ms)?;

    // For subsequent segments, filter descendants
    for seg in selector.segments.iter().skip(1) {
        let mut next_candidates = Vec::new();

        for candidate in &candidates {
            let matches = find_by_segment(automation, candidate, seg, 0)?;
            next_candidates.extend(matches);
        }

        candidates = next_candidates;

        if !find_all && !candidates.is_empty() {
            break;
        }
    }

    // Convert to UiaElement
    for candidate in candidates {
        results.push(element_to_uia(&candidate, 0));
        if !find_all {
            break;
        }
    }

    Ok(results)
}

/// Find elements matching a single selector segment
fn find_by_segment(
    automation: &UIAutomation,
    root: &UIElement,
    seg: &SelectorSegment,
    timeout_ms: u64,
) -> Result<Vec<UIElement>, uiautomation::Error> {
    // Note: seg.is_direct_child would use TreeScope::Children vs Descendants
    // but UIMatcher doesn't support scope configuration directly - it uses depth instead

    // Build condition based on segment
    let mut matcher = uiautomation::UIMatcher::new(automation.clone());

    if timeout_ms > 0 {
        matcher = matcher.timeout(timeout_ms);
    } else {
        matcher = matcher.timeout(0); // Don't retry
    }

    // Use depth of 20 to search deeper in the tree (default is 7)
    matcher = matcher.depth(20);

    matcher = matcher.from(root.clone());

    // Set control type if specified and not wildcard
    if let Some(ref ct) = seg.control_type {
        if ct != "*" {
            matcher = matcher.classname(ct); // Note: control_type filter would be better but UIMatcher uses classname
        }
    }

    // Note: UIMatcher in uiautomation 0.24 doesn't support automation_id filtering
    // We'll filter by automation_id manually after getting results

    // For class name, we need to filter manually since UIMatcher uses classname differently
    // Get all matching elements and then filter

    let elements = match matcher.find_all() {
        Ok(elems) => elems,
        Err(_) => return Ok(Vec::new()),
    };

    // Apply additional filters
    let mut results = Vec::new();

    for elem in elements {
        if matches_segment(&elem, seg) {
            results.push(elem);
        }
    }

    Ok(results)
}

/// Check if an element matches a selector segment's additional criteria
fn matches_segment(elem: &UIElement, seg: &SelectorSegment) -> bool {
    // Check automation ID
    if let Some(ref expected_id) = seg.automation_id {
        let actual = elem.get_automation_id().unwrap_or_default();
        if actual != *expected_id {
            return false;
        }
    }

    // Check class name
    if let Some(ref expected_class) = seg.class_name {
        let actual = elem.get_classname().unwrap_or_default();
        if !actual.contains(expected_class) {
            return false;
        }
    }

    // Check attribute matchers
    for attr in &seg.attributes {
        let actual_value = get_element_attribute(elem, &attr.name);
        if !attr.matches(&actual_value) {
            return false;
        }
    }

    true
}

/// Get an attribute value from an element by attribute name
fn get_element_attribute(elem: &UIElement, attr_name: &str) -> String {
    match attr_name.to_lowercase().as_str() {
        "name" => elem.get_name().unwrap_or_default(),
        "automationid" | "automation_id" | "id" => elem.get_automation_id().unwrap_or_default(),
        "classname" | "class" => elem.get_classname().unwrap_or_default(),
        "value" => elem
            .get_pattern::<UIValuePattern>()
            .ok()
            .and_then(|p| p.get_value().ok())
            .unwrap_or_default(),
        "controltype" | "control_type" | "type" => elem
            .get_control_type()
            .map(|ct| format!("{:?}", ct))
            .unwrap_or_default(),
        "enabled" => elem.is_enabled().unwrap_or(true).to_string(),
        "visible" | "onscreen" => (!elem.is_offscreen().unwrap_or(false)).to_string(),
        _ => String::new(),
    }
}
