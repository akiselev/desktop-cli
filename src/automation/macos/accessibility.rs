//! macOS accessibility tree via AXUIElement API
//!
//! The macOS Accessibility API provides:
//! - Element tree traversal via AXUIElementRef
//! - Element properties (title, role, value)
//! - Actions (press, increment, etc.)
//! - State information (enabled, focused)

use crate::automation::macos::window::WindowId;
use crate::error::{DesktopCliError, Result};
use crate::rpc::types::{PatternResult, TreeDumpOptions, UiaElement};
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use std::ffi::c_void;
use std::ptr;

// Accessibility API types and functions
#[repr(C)]
pub struct __AXUIElement(c_void);
pub type AXUIElementRef = *mut __AXUIElement;

#[repr(C)]
pub struct __AXValue(c_void);
pub type AXValueRef = *mut __AXValue;

pub type AXError = i32;

// AXError constants
pub const kAXErrorSuccess: AXError = 0;
pub const kAXErrorFailure: AXError = -25200;
pub const kAXErrorAttributeUnsupported: AXError = -25205;
pub const kAXErrorNoValue: AXError = -25212;
pub const kAXErrorAPIDisabled: AXError = -25211;

// AXValue types
pub const kAXValueTypeCGPoint: u32 = 1;
pub const kAXValueTypeCGSize: u32 = 2;
pub const kAXValueTypeCGRect: u32 = 3;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateApplication(pid: libc::pid_t) -> AXUIElementRef;
    fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: core_foundation::string::CFStringRef,
        value: *mut core_foundation::base::CFTypeRef,
    ) -> AXError;
    fn AXUIElementCopyAttributeNames(
        element: AXUIElementRef,
        names: *mut core_foundation::array::CFArrayRef,
    ) -> AXError;
    fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: core_foundation::string::CFStringRef,
        value: core_foundation::base::CFTypeRef,
    ) -> AXError;
    fn AXUIElementPerformAction(
        element: AXUIElementRef,
        action: core_foundation::string::CFStringRef,
    ) -> AXError;
    fn AXUIElementCopyActionNames(
        element: AXUIElementRef,
        names: *mut core_foundation::array::CFArrayRef,
    ) -> AXError;
    fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut libc::pid_t) -> AXError;
    fn AXIsProcessTrusted() -> bool;
    fn AXValueGetValue(
        value: AXValueRef,
        value_type: u32,
        value_ptr: *mut c_void,
    ) -> bool;
    fn AXValueGetType(value: AXValueRef) -> u32;
}

// Accessibility attribute constants
const AX_ROLE: &str = "AXRole";
const AX_ROLE_DESCRIPTION: &str = "AXRoleDescription";
const AX_TITLE: &str = "AXTitle";
const AX_VALUE: &str = "AXValue";
const AX_DESCRIPTION: &str = "AXDescription";
const AX_CHILDREN: &str = "AXChildren";
const AX_PARENT: &str = "AXParent";
const AX_ENABLED: &str = "AXEnabled";
const AX_FOCUSED: &str = "AXFocused";
const AX_POSITION: &str = "AXPosition";
const AX_SIZE: &str = "AXSize";
const AX_WINDOWS: &str = "AXWindows";
const AX_MAIN_WINDOW: &str = "AXMainWindow";
const AX_FOCUSED_WINDOW: &str = "AXFocusedWindow";
const AX_FOCUSED_UI_ELEMENT: &str = "AXFocusedUIElement";
const AX_IDENTIFIER: &str = "AXIdentifier";

// Accessibility action constants
const AX_PRESS: &str = "AXPress";
const AX_INCREMENT: &str = "AXIncrement";
const AX_DECREMENT: &str = "AXDecrement";
const AX_CONFIRM: &str = "AXConfirm";
const AX_CANCEL: &str = "AXCancel";
const AX_RAISE: &str = "AXRaise";

/// CGPoint structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

/// CGSize structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CGSize {
    pub width: f64,
    pub height: f64,
}

/// CGRect structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

/// Wrapper for AXUIElementRef with automatic cleanup
pub struct AXElement {
    element: AXUIElementRef,
}

impl AXElement {
    /// Create a new AXElement from a raw reference
    pub fn from_ref(element: AXUIElementRef) -> Option<Self> {
        if element.is_null() {
            None
        } else {
            Some(Self { element })
        }
    }

    /// Create an AXElement for a specific application by PID
    pub fn from_pid(pid: u32) -> Option<Self> {
        let element = unsafe { AXUIElementCreateApplication(pid as libc::pid_t) };
        Self::from_ref(element)
    }

    /// Create a system-wide AXElement
    pub fn system_wide() -> Option<Self> {
        let element = unsafe { AXUIElementCreateSystemWide() };
        Self::from_ref(element)
    }

    /// Get the raw AXUIElementRef
    pub fn as_ref(&self) -> AXUIElementRef {
        self.element
    }

    /// Get a string attribute
    pub fn get_string_attribute(&self, attr: &str) -> Option<String> {
        unsafe {
            let attr_str = CFString::new(attr);
            let mut value: core_foundation::base::CFTypeRef = ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                self.element,
                attr_str.as_concrete_TypeRef(),
                &mut value,
            );

            if result != kAXErrorSuccess || value.is_null() {
                return None;
            }

            let cf_type: CFType = CFType::wrap_under_create_rule(value);

            // Try to get as string
            if let Some(cf_string) = cf_type.downcast::<CFString>() {
                return Some(cf_string.to_string());
            }

            // Try to get as number and convert
            if let Some(cf_number) = cf_type.downcast::<CFNumber>() {
                if let Some(n) = cf_number.to_i64() {
                    return Some(n.to_string());
                }
                if let Some(n) = cf_number.to_f64() {
                    return Some(n.to_string());
                }
            }

            None
        }
    }

    /// Get a boolean attribute
    pub fn get_bool_attribute(&self, attr: &str) -> Option<bool> {
        unsafe {
            let attr_str = CFString::new(attr);
            let mut value: core_foundation::base::CFTypeRef = ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                self.element,
                attr_str.as_concrete_TypeRef(),
                &mut value,
            );

            if result != kAXErrorSuccess || value.is_null() {
                return None;
            }

            let cf_type: CFType = CFType::wrap_under_create_rule(value);

            if let Some(cf_bool) = cf_type.downcast::<CFBoolean>() {
                return Some(cf_bool.into());
            }

            None
        }
    }

    /// Get the position (AXPosition) of the element
    pub fn get_position(&self) -> Option<CGPoint> {
        unsafe {
            let attr_str = CFString::new(AX_POSITION);
            let mut value: core_foundation::base::CFTypeRef = ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                self.element,
                attr_str.as_concrete_TypeRef(),
                &mut value,
            );

            if result != kAXErrorSuccess || value.is_null() {
                return None;
            }

            let value_ref = value as AXValueRef;
            let value_type = AXValueGetType(value_ref);

            if value_type != kAXValueTypeCGPoint {
                core_foundation::base::CFRelease(value);
                return None;
            }

            let mut point = CGPoint::default();
            let success = AXValueGetValue(
                value_ref,
                kAXValueTypeCGPoint,
                &mut point as *mut _ as *mut c_void,
            );

            core_foundation::base::CFRelease(value);

            if success {
                Some(point)
            } else {
                None
            }
        }
    }

    /// Get the size (AXSize) of the element
    pub fn get_size(&self) -> Option<CGSize> {
        unsafe {
            let attr_str = CFString::new(AX_SIZE);
            let mut value: core_foundation::base::CFTypeRef = ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                self.element,
                attr_str.as_concrete_TypeRef(),
                &mut value,
            );

            if result != kAXErrorSuccess || value.is_null() {
                return None;
            }

            let value_ref = value as AXValueRef;
            let value_type = AXValueGetType(value_ref);

            if value_type != kAXValueTypeCGSize {
                core_foundation::base::CFRelease(value);
                return None;
            }

            let mut size = CGSize::default();
            let success = AXValueGetValue(
                value_ref,
                kAXValueTypeCGSize,
                &mut size as *mut _ as *mut c_void,
            );

            core_foundation::base::CFRelease(value);

            if success {
                Some(size)
            } else {
                None
            }
        }
    }

    /// Get the bounds (position + size) of the element
    pub fn get_bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let pos = self.get_position()?;
        let size = self.get_size()?;

        Some((
            pos.x as i32,
            pos.y as i32,
            size.width as i32,
            size.height as i32,
        ))
    }

    /// Get child elements
    pub fn get_children(&self) -> Vec<AXElement> {
        unsafe {
            let attr_str = CFString::new(AX_CHILDREN);
            let mut value: core_foundation::base::CFTypeRef = ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                self.element,
                attr_str.as_concrete_TypeRef(),
                &mut value,
            );

            if result != kAXErrorSuccess || value.is_null() {
                return Vec::new();
            }

            let cf_type: CFType = CFType::wrap_under_create_rule(value);

            let array: CFArray<CFType> = match cf_type.downcast() {
                Some(a) => a,
                None => return Vec::new(),
            };

            let mut children = Vec::new();
            for i in 0..array.len() {
                let item = array.get(i);
                let element_ref = item.as_CFTypeRef() as AXUIElementRef;

                // Retain the element since we're going to store it
                core_foundation::base::CFRetain(element_ref as core_foundation::base::CFTypeRef);

                if let Some(elem) = AXElement::from_ref(element_ref) {
                    children.push(elem);
                }
            }

            children
        }
    }

    /// Get available actions
    pub fn get_actions(&self) -> Vec<String> {
        unsafe {
            let mut names: core_foundation::array::CFArrayRef = ptr::null_mut();
            let result = AXUIElementCopyActionNames(self.element, &mut names);

            if result != kAXErrorSuccess || names.is_null() {
                return Vec::new();
            }

            let array: CFArray<CFString> = CFArray::wrap_under_create_rule(names);

            let mut actions = Vec::new();
            for i in 0..array.len() {
                let item = array.get(i);
                actions.push(item.to_string());
            }

            actions
        }
    }

    /// Perform an action
    pub fn perform_action(&self, action: &str) -> Result<()> {
        unsafe {
            let action_str = CFString::new(action);
            let result = AXUIElementPerformAction(self.element, action_str.as_concrete_TypeRef());

            if result == kAXErrorSuccess {
                Ok(())
            } else {
                Err(DesktopCliError::AutomationError(format!(
                    "Failed to perform action '{}': error {}",
                    action, result
                )))
            }
        }
    }

    /// Set an attribute value
    pub fn set_string_attribute(&self, attr: &str, value: &str) -> Result<()> {
        unsafe {
            let attr_str = CFString::new(attr);
            let value_str = CFString::new(value);

            let result = AXUIElementSetAttributeValue(
                self.element,
                attr_str.as_concrete_TypeRef(),
                value_str.as_CFTypeRef(),
            );

            if result == kAXErrorSuccess {
                Ok(())
            } else {
                Err(DesktopCliError::AutomationError(format!(
                    "Failed to set attribute '{}': error {}",
                    attr, result
                )))
            }
        }
    }

    /// Get the role of this element
    pub fn role(&self) -> String {
        self.get_string_attribute(AX_ROLE).unwrap_or_default()
    }

    /// Get the title of this element
    pub fn title(&self) -> String {
        self.get_string_attribute(AX_TITLE).unwrap_or_default()
    }

    /// Get the value of this element
    pub fn value(&self) -> Option<String> {
        self.get_string_attribute(AX_VALUE)
    }

    /// Get the description of this element
    pub fn description(&self) -> String {
        self.get_string_attribute(AX_DESCRIPTION).unwrap_or_default()
    }

    /// Get the role description of this element
    pub fn role_description(&self) -> String {
        self.get_string_attribute(AX_ROLE_DESCRIPTION).unwrap_or_default()
    }

    /// Get the identifier of this element
    pub fn identifier(&self) -> String {
        self.get_string_attribute(AX_IDENTIFIER).unwrap_or_default()
    }

    /// Check if the element is enabled
    pub fn is_enabled(&self) -> bool {
        self.get_bool_attribute(AX_ENABLED).unwrap_or(true)
    }

    /// Check if the element is focused
    pub fn is_focused(&self) -> bool {
        self.get_bool_attribute(AX_FOCUSED).unwrap_or(false)
    }

    /// Get the windows for an application element
    pub fn get_windows(&self) -> Vec<AXElement> {
        unsafe {
            let attr_str = CFString::new(AX_WINDOWS);
            let mut value: core_foundation::base::CFTypeRef = ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                self.element,
                attr_str.as_concrete_TypeRef(),
                &mut value,
            );

            if result != kAXErrorSuccess || value.is_null() {
                return Vec::new();
            }

            let cf_type: CFType = CFType::wrap_under_create_rule(value);

            let array: CFArray<CFType> = match cf_type.downcast() {
                Some(a) => a,
                None => return Vec::new(),
            };

            let mut windows = Vec::new();
            for i in 0..array.len() {
                let item = array.get(i);
                let element_ref = item.as_CFTypeRef() as AXUIElementRef;
                core_foundation::base::CFRetain(element_ref as core_foundation::base::CFTypeRef);

                if let Some(elem) = AXElement::from_ref(element_ref) {
                    windows.push(elem);
                }
            }

            windows
        }
    }

    /// Get the main window for an application element
    pub fn get_main_window(&self) -> Option<AXElement> {
        unsafe {
            let attr_str = CFString::new(AX_MAIN_WINDOW);
            let mut value: core_foundation::base::CFTypeRef = ptr::null_mut();

            let result = AXUIElementCopyAttributeValue(
                self.element,
                attr_str.as_concrete_TypeRef(),
                &mut value,
            );

            if result != kAXErrorSuccess || value.is_null() {
                return None;
            }

            // Retain the element since we're storing it
            core_foundation::base::CFRetain(value);
            AXElement::from_ref(value as AXUIElementRef)
        }
    }
}

impl Clone for AXElement {
    fn clone(&self) -> Self {
        unsafe {
            core_foundation::base::CFRetain(self.element as core_foundation::base::CFTypeRef);
        }
        Self {
            element: self.element,
        }
    }
}

impl Drop for AXElement {
    fn drop(&mut self) {
        if !self.element.is_null() {
            unsafe {
                core_foundation::base::CFRelease(self.element as core_foundation::base::CFTypeRef);
            }
        }
    }
}

/// Check if accessibility is enabled
pub fn is_accessibility_enabled() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Convert AX role to cross-platform control type
pub fn role_to_control_type(role: &str) -> String {
    match role {
        "AXButton" => "Button".to_string(),
        "AXRadioButton" => "RadioButton".to_string(),
        "AXCheckBox" => "CheckBox".to_string(),
        "AXPopUpButton" => "ComboBox".to_string(),
        "AXComboBox" => "ComboBox".to_string(),
        "AXTextField" => "Edit".to_string(),
        "AXTextArea" => "Edit".to_string(),
        "AXSecureTextField" => "PasswordEdit".to_string(),
        "AXStaticText" => "Text".to_string(),
        "AXImage" => "Image".to_string(),
        "AXLink" => "Hyperlink".to_string(),
        "AXList" => "List".to_string(),
        "AXTable" => "Table".to_string(),
        "AXOutline" => "Tree".to_string(),
        "AXRow" => "ListItem".to_string(),
        "AXCell" => "TableCell".to_string(),
        "AXMenu" => "Menu".to_string(),
        "AXMenuBar" => "MenuBar".to_string(),
        "AXMenuItem" => "MenuItem".to_string(),
        "AXMenuButton" => "MenuButton".to_string(),
        "AXToolbar" => "ToolBar".to_string(),
        "AXTabGroup" => "Tab".to_string(),
        "AXTab" | "AXRadioGroup" => "TabItem".to_string(),
        "AXSlider" => "Slider".to_string(),
        "AXProgressIndicator" => "ProgressBar".to_string(),
        "AXScrollBar" => "ScrollBar".to_string(),
        "AXScrollArea" => "ScrollPane".to_string(),
        "AXSplitGroup" => "SplitPane".to_string(),
        "AXGroup" => "Group".to_string(),
        "AXWindow" => "Window".to_string(),
        "AXSheet" | "AXDialog" => "Dialog".to_string(),
        "AXApplication" => "Application".to_string(),
        _ => role.trim_start_matches("AX").to_string(),
    }
}

/// Convert AXElement to UiaElement
pub fn element_to_uia(elem: &AXElement, depth: u32) -> UiaElement {
    let role = elem.role();
    let title = elem.title();
    let value = elem.value();
    let description = elem.description();
    let identifier = elem.identifier();
    let bounds = elem.get_bounds().unwrap_or((0, 0, 0, 0));
    let is_enabled = elem.is_enabled();
    let actions = elem.get_actions();

    // Determine patterns from available actions
    let mut patterns = Vec::new();
    if actions.contains(&AX_PRESS.to_string()) {
        patterns.push("Invoke".to_string());
    }
    if actions.contains(&AX_INCREMENT.to_string()) || actions.contains(&AX_DECREMENT.to_string()) {
        patterns.push("RangeValue".to_string());
    }
    if value.is_some() {
        patterns.push("Value".to_string());
    }
    if role == "AXCheckBox" || role == "AXRadioButton" {
        patterns.push("Toggle".to_string());
    }
    if role == "AXTextField" || role == "AXTextArea" {
        patterns.push("Text".to_string());
    }

    // Use title, description, or role as name
    let name = if !title.is_empty() {
        title
    } else if !description.is_empty() {
        description
    } else {
        String::new()
    };

    UiaElement {
        id: format!("{:p}", elem.as_ref()),
        control_type: role_to_control_type(&role),
        localized_type: elem.role_description(),
        name,
        automation_id: identifier,
        class_name: role,
        value,
        bounds: [bounds.0, bounds.1, bounds.2, bounds.3],
        is_enabled,
        is_offscreen: false, // macOS doesn't have a direct equivalent
        patterns,
        depth,
        children: Vec::new(),
    }
}

/// Dump the accessibility tree starting from an element
pub fn dump_tree(elem: &AXElement, options: &TreeDumpOptions) -> Result<UiaElement> {
    dump_element_recursive(elem, 0, options)
}

fn dump_element_recursive(
    elem: &AXElement,
    depth: u32,
    options: &TreeDumpOptions,
) -> Result<UiaElement> {
    let mut uia_elem = element_to_uia(elem, depth);

    // Recurse into children if within depth limit
    if depth < options.max_depth {
        let children = elem.get_children();
        let mut child_count = 0;

        for child in children {
            // Check list item cap
            if options.max_list_items > 0 && child_count >= options.max_list_items {
                let mut placeholder = UiaElement::default();
                placeholder.name = "... (more items)".to_string();
                placeholder.depth = depth + 1;
                uia_elem.children.push(placeholder);
                break;
            }

            if let Ok(child_elem) = dump_element_recursive(&child, depth + 1, options) {
                // Apply pruning rules
                let should_include = if options.prune_empty
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
            }
        }
    }

    Ok(uia_elem)
}

/// Get the AXElement for a window by its CGWindow ID
pub fn get_element_for_window(window_id: WindowId) -> Result<AXElement> {
    // First, we need to find the PID of the window owner
    use crate::automation::macos::window::get_window_info;

    let info = get_window_info(window_id)?;

    // Create AXElement for the application
    let app = AXElement::from_pid(info.pid).ok_or_else(|| {
        DesktopCliError::AutomationError(format!(
            "Failed to create AXElement for PID {}",
            info.pid
        ))
    })?;

    // Find the window with matching title
    let windows = app.get_windows();
    for window in windows {
        let window_title = window.title();
        if window_title == info.title {
            return Ok(window);
        }
    }

    // If no exact match, try main window
    if let Some(main_window) = app.get_main_window() {
        return Ok(main_window);
    }

    // Return the app element as fallback
    Ok(app)
}

/// Pattern operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternOp {
    Invoke,
    GetValue,
    SetValue,
    Toggle,
    Focus,
    Increment,
    Decrement,
}

impl PatternOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "invoke" | "click" | "press" => Some(Self::Invoke),
            "get-value" | "getvalue" | "value" => Some(Self::GetValue),
            "set-value" | "setvalue" => Some(Self::SetValue),
            "toggle" => Some(Self::Toggle),
            "focus" => Some(Self::Focus),
            "increment" => Some(Self::Increment),
            "decrement" => Some(Self::Decrement),
            _ => None,
        }
    }
}

/// Execute a pattern operation on an element
pub fn execute_pattern(
    elem: &AXElement,
    pattern: PatternOp,
    value: Option<&str>,
) -> PatternResult {
    match pattern {
        PatternOp::Invoke => match elem.perform_action(AX_PRESS) {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(e.to_string()),
        },
        PatternOp::GetValue => match elem.value() {
            Some(v) => PatternResult::ok_with_value(v),
            None => PatternResult::err("No value available"),
        },
        PatternOp::SetValue => {
            let v = value.unwrap_or("");
            match elem.set_string_attribute(AX_VALUE, v) {
                Ok(()) => PatternResult::ok(),
                Err(e) => PatternResult::err(e.to_string()),
            }
        }
        PatternOp::Toggle => match elem.perform_action(AX_PRESS) {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(e.to_string()),
        },
        PatternOp::Focus => {
            // Focus by setting AXFocused or performing AXRaise
            if elem.perform_action(AX_RAISE).is_ok() {
                PatternResult::ok()
            } else {
                PatternResult::err("Could not focus element")
            }
        }
        PatternOp::Increment => match elem.perform_action(AX_INCREMENT) {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(e.to_string()),
        },
        PatternOp::Decrement => match elem.perform_action(AX_DECREMENT) {
            Ok(()) => PatternResult::ok(),
            Err(e) => PatternResult::err(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_to_control_type() {
        assert_eq!(role_to_control_type("AXButton"), "Button");
        assert_eq!(role_to_control_type("AXTextField"), "Edit");
        assert_eq!(role_to_control_type("AXWindow"), "Window");
    }

    #[test]
    fn test_pattern_op_from_str() {
        assert_eq!(PatternOp::from_str("invoke"), Some(PatternOp::Invoke));
        assert_eq!(PatternOp::from_str("click"), Some(PatternOp::Invoke));
        assert_eq!(PatternOp::from_str("get-value"), Some(PatternOp::GetValue));
        assert_eq!(PatternOp::from_str("invalid"), None);
    }
}
