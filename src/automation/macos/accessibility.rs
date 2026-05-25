//! Cocoa Accessibility tree operations

use crate::error::Result;
use crate::rpc::types::UiaElement;
use accessibility_sys::{
    kAXChildrenAttribute, kAXPositionAttribute, kAXPressAction, kAXRoleAttribute,
    kAXSizeAttribute, kAXTitleAttribute, kAXValueAttribute, AXUIElementCopyAttributeValue,
    AXUIElementCreateApplication, AXUIElementPerformAction, AXUIElementRef,
};
use core_foundation::base::{CFTypeRef, TCFType};
use core_foundation::string::CFString;
use std::collections::HashSet;

#[cfg(target_os = "macos")]
pub fn dump_tree(window_ref: &str, max_depth: u32) -> Result<UiaElement> {
    if !super::permissions::check_accessibility_permission() {
        return Err(crate::error::DesktopCliError::AutomationError(
            "Desktop automation requires accessibility permissions. Grant access in System Settings > Privacy & Security > Accessibility.".to_string()
        ));
    }

    let pid = parse_window_ref(window_ref)?;

    unsafe {
        let app_ref = AXUIElementCreateApplication(pid as i32);
        if app_ref.is_null() {
            return Err(crate::error::DesktopCliError::AutomationError(
                "Failed to create AXUIElement for application".to_string(),
            ));
        }

        let mut visited = HashSet::new();
        let result = dump_element_recursive(app_ref, 0, max_depth, &mut visited);

        core_foundation::base::CFRelease(app_ref as CFTypeRef);

        result
    }
}

fn parse_window_ref(window_ref: &str) -> Result<u32> {
    if let Some(stripped) = window_ref.strip_prefix("0x") {
        u32::from_str_radix(stripped, 16)
            .map_err(|_| crate::error::DesktopCliError::AutomationError(
                format!("Invalid window reference: {}", window_ref)
            ))
    } else {
        window_ref.parse::<u32>()
            .map_err(|_| crate::error::DesktopCliError::AutomationError(
                format!("Invalid window reference: {}", window_ref)
            ))
    }
    .and_then(|_window_id| {
        super::window::get_window_info_by_id(_window_id).map(|info| info.pid)
    })
}

unsafe fn dump_element_recursive(
    element: AXUIElementRef,
    depth: u32,
    max_depth: u32,
    visited: &mut HashSet<usize>,
) -> Result<UiaElement> {
    let elem_ptr = element as usize;

    if visited.contains(&elem_ptr) {
        let mut circular = UiaElement::default();
        circular.name = "[circular]".to_string();
        circular.depth = depth;
        return Ok(circular);
    }

    visited.insert(elem_ptr);

    let mut uia_elem = element_to_uia(element, depth)?;

    // Uses max_depth=5 to prevent stack overflow on circular refs. 5 levels covers typical UI hierarchies per Apple HIG. See Decision Log.
    if depth < max_depth {
        if let Ok(children) = get_children(element) {
            for child_ref in children {
                match dump_element_recursive(child_ref, depth + 1, max_depth, visited) {
                    Ok(child_elem) => {
                        if child_elem.name != "[circular]" {
                            uia_elem.children.push(child_elem);
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }

    visited.remove(&elem_ptr);

    Ok(uia_elem)
}

unsafe fn element_to_uia(element: AXUIElementRef, depth: u32) -> Result<UiaElement> {
    let role = get_attribute_string(element, kAXRoleAttribute).unwrap_or_default();
    let name = get_attribute_string(element, kAXTitleAttribute).unwrap_or_default();
    let value = get_attribute_string(element, kAXValueAttribute);

    let control_type = super::roles::map_role(&role);
    let (x, y, width, height) = get_bounds(element);

    let id = format!("{:p}", element);

    let patterns = detect_patterns(element, &role);

    Ok(UiaElement {
        id,
        control_type,
        localized_type: role.clone(),
        name,
        automation_id: String::new(),
        class_name: role,
        value,
        bounds: [x, y, width, height],
        is_enabled: true,
        is_offscreen: false,
        patterns,
        depth,
        children: Vec::new(),
    })
}

/// Extracts a string attribute from an AXUIElement.
/// The `attribute` parameter is a `&str` (as defined in accessibility-sys 0.1).
unsafe fn get_attribute_string(element: AXUIElementRef, attribute: &str) -> Option<String> {
    let attr_cf = CFString::new(attribute);
    let mut value: CFTypeRef = std::ptr::null();
    let result = AXUIElementCopyAttributeValue(element, attr_cf.as_concrete_TypeRef(), &mut value);

    if result == 0 && !value.is_null() {
        let cf_string = CFString::wrap_under_create_rule(value as _);
        Some(cf_string.to_string())
    } else {
        if !value.is_null() {
            core_foundation::base::CFRelease(value);
        }
        None
    }
}

unsafe fn get_children(element: AXUIElementRef) -> Result<Vec<AXUIElementRef>> {
    let attr_cf = CFString::new(kAXChildrenAttribute);
    let mut value: CFTypeRef = std::ptr::null();
    let result = AXUIElementCopyAttributeValue(element, attr_cf.as_concrete_TypeRef(), &mut value);

    if result != 0 || value.is_null() {
        return Ok(Vec::new());
    }

    // Use raw CFArray access to avoid FromVoid trait bound issues with AXUIElementRef
    use core_foundation_sys::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
    let arr_ref = value as core_foundation_sys::array::CFArrayRef;
    let count = CFArrayGetCount(arr_ref);
    let mut children = Vec::new();

    for i in 0..count {
        let child = CFArrayGetValueAtIndex(arr_ref, i) as AXUIElementRef;
        if !child.is_null() {
            children.push(child);
        }
    }

    core_foundation::base::CFRelease(value);
    Ok(children)
}

unsafe fn get_bounds(element: AXUIElementRef) -> (i32, i32, i32, i32) {
    let pos_attr = CFString::new(kAXPositionAttribute);
    let size_attr = CFString::new(kAXSizeAttribute);
    let mut pos_value: CFTypeRef = std::ptr::null();
    let mut size_value: CFTypeRef = std::ptr::null();

    let pos_result = AXUIElementCopyAttributeValue(element, pos_attr.as_concrete_TypeRef(), &mut pos_value);
    let size_result = AXUIElementCopyAttributeValue(element, size_attr.as_concrete_TypeRef(), &mut size_value);

    let (x, y) = if pos_result == 0 && !pos_value.is_null() {
        extract_point(pos_value)
    } else {
        (0, 0)
    };

    let (width, height) = if size_result == 0 && !size_value.is_null() {
        extract_size(size_value)
    } else {
        (0, 0)
    };

    if !pos_value.is_null() {
        core_foundation::base::CFRelease(pos_value);
    }
    if !size_value.is_null() {
        core_foundation::base::CFRelease(size_value);
    }

    (x, y, width, height)
}

unsafe fn extract_point(value: CFTypeRef) -> (i32, i32) {
    use core_foundation::number::CFNumber;
    use core_foundation_sys::dictionary::CFDictionaryGetValue;

    let dict_ref = value as core_foundation_sys::dictionary::CFDictionaryRef;
    let x_key = CFString::new("x");
    let y_key = CFString::new("y");

    let x_val = CFDictionaryGetValue(dict_ref, x_key.as_concrete_TypeRef() as _);
    let x = if !x_val.is_null() {
        CFNumber::wrap_under_get_rule(x_val as _).to_i32().unwrap_or(0)
    } else {
        0
    };

    let y_val = CFDictionaryGetValue(dict_ref, y_key.as_concrete_TypeRef() as _);
    let y = if !y_val.is_null() {
        CFNumber::wrap_under_get_rule(y_val as _).to_i32().unwrap_or(0)
    } else {
        0
    };

    (x, y)
}

unsafe fn extract_size(value: CFTypeRef) -> (i32, i32) {
    use core_foundation::number::CFNumber;
    use core_foundation_sys::dictionary::CFDictionaryGetValue;

    let dict_ref = value as core_foundation_sys::dictionary::CFDictionaryRef;
    let w_key = CFString::new("w");
    let h_key = CFString::new("h");

    let w_val = CFDictionaryGetValue(dict_ref, w_key.as_concrete_TypeRef() as _);
    let w = if !w_val.is_null() {
        CFNumber::wrap_under_get_rule(w_val as _).to_i32().unwrap_or(0)
    } else {
        0
    };

    let h_val = CFDictionaryGetValue(dict_ref, h_key.as_concrete_TypeRef() as _);
    let h = if !h_val.is_null() {
        CFNumber::wrap_under_get_rule(h_val as _).to_i32().unwrap_or(0)
    } else {
        0
    };

    (w, h)
}

fn detect_patterns(_element: AXUIElementRef, role: &str) -> Vec<String> {
    let mut patterns = Vec::new();

    match role {
        "AXButton" | "AXMenuItem" => {
            patterns.push("Invoke".to_string());
        }
        "AXTextField" | "AXTextArea" => {
            patterns.push("Value".to_string());
        }
        "AXCheckBox" => {
            patterns.push("Toggle".to_string());
        }
        _ => {}
    }

    patterns
}

#[cfg(target_os = "macos")]
pub fn find_elements(
    window_ref: &str,
    selector: &str,
    find_all: bool,
) -> Result<Vec<UiaElement>> {
    let tree = dump_tree(window_ref, 5)?;
    let mut results = Vec::new();

    find_elements_recursive(&tree, selector, find_all, &mut results);

    Ok(results)
}

fn find_elements_recursive(
    element: &UiaElement,
    selector: &str,
    find_all: bool,
    results: &mut Vec<UiaElement>,
) {
    if matches_selector(element, selector) {
        results.push(element.clone());
        if !find_all {
            return;
        }
    }

    for child in &element.children {
        find_elements_recursive(child, selector, find_all, results);
        if !find_all && !results.is_empty() {
            return;
        }
    }
}

fn matches_selector(element: &UiaElement, selector: &str) -> bool {
    let selector_lower = selector.to_lowercase();

    if element.control_type.to_lowercase() == selector_lower {
        return true;
    }

    if element.name.to_lowercase().contains(&selector_lower) {
        return true;
    }

    false
}

#[cfg(target_os = "macos")]
pub fn invoke_pattern(
    window_ref: &str,
    selector: &str,
    pattern: &str,
    value: Option<&str>,
) -> Result<crate::rpc::types::PatternResult> {
    use crate::rpc::types::PatternResult;

    if !super::permissions::check_accessibility_permission() {
        return Ok(PatternResult::err(
            "Desktop automation requires accessibility permissions. Grant access in System Settings > Privacy & Security > Accessibility."
        ));
    }

    let elements = find_elements(window_ref, selector, false)?;

    if elements.is_empty() {
        return Ok(PatternResult::err(format!(
            "Element not found: {}",
            selector
        )));
    }

    let element = &elements[0];

    match pattern.to_lowercase().as_str() {
        "invoke" | "click" => invoke_element(&element.id),
        "get-value" | "getvalue" | "value" => get_value_pattern(&element.id),
        "set-value" | "setvalue" => {
            if let Some(val) = value {
                set_value_pattern(&element.id, val)
            } else {
                Ok(PatternResult::err("set-value requires a value parameter"))
            }
        }
        _ => Ok(PatternResult::err(format!(
            "Unsupported pattern: {}",
            pattern
        ))),
    }
}

fn invoke_element(element_id: &str) -> Result<crate::rpc::types::PatternResult> {
    use crate::rpc::types::PatternResult;

    let ptr: usize = usize::from_str_radix(element_id.trim_start_matches("0x"), 16)
        .map_err(|_| {
            crate::error::DesktopCliError::AutomationError(
                "Invalid element ID".to_string(),
            )
        })?;

    let element = ptr as AXUIElementRef;

    unsafe {
        let action = CFString::new("AXPress");
        let result = AXUIElementPerformAction(element, action.as_concrete_TypeRef());

        if result == 0 {
            Ok(PatternResult::ok())
        } else {
            Ok(PatternResult::err(format!(
                "AXPress action failed with code: {}",
                result
            )))
        }
    }
}

fn get_value_pattern(element_id: &str) -> Result<crate::rpc::types::PatternResult> {
    use crate::rpc::types::PatternResult;

    let ptr: usize = usize::from_str_radix(element_id.trim_start_matches("0x"), 16)
        .map_err(|_| {
            crate::error::DesktopCliError::AutomationError(
                "Invalid element ID".to_string(),
            )
        })?;

    let element = ptr as AXUIElementRef;

    unsafe {
        if let Some(value) = get_attribute_string(element, kAXValueAttribute) {
            Ok(PatternResult::ok_with_value(value))
        } else {
            Ok(PatternResult::err("Failed to get value"))
        }
    }
}

fn set_value_pattern(element_id: &str, _value: &str) -> Result<crate::rpc::types::PatternResult> {
    use crate::rpc::types::PatternResult;

    let _ptr: usize = usize::from_str_radix(element_id.trim_start_matches("0x"), 16)
        .map_err(|_| {
            crate::error::DesktopCliError::AutomationError(
                "Invalid element ID".to_string(),
            )
        })?;

    Ok(PatternResult::err(
        "set-value not yet implemented for macOS",
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn dump_tree(_window_ref: &str, _max_depth: u32) -> Result<UiaElement> {
    Err(crate::error::DesktopCliError::Platform(
        "macOS not supported on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn find_elements(
    _window_ref: &str,
    _selector: &str,
    _find_all: bool,
) -> Result<Vec<UiaElement>> {
    Err(crate::error::DesktopCliError::Platform(
        "macOS not supported on this platform".to_string(),
    ))
}

#[cfg(not(target_os = "macos"))]
pub fn invoke_pattern(
    _window_ref: &str,
    _selector: &str,
    _pattern: &str,
    _value: Option<&str>,
) -> Result<crate::rpc::types::PatternResult> {
    Err(crate::error::DesktopCliError::Platform(
        "macOS not supported on this platform".to_string(),
    ))
}
