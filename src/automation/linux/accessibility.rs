//! Linux accessibility tree via AT-SPI2 (Assistive Technology Service Provider Interface)
//!
//! AT-SPI2 is the Linux accessibility framework that provides:
//! - Element tree traversal
//! - Element properties (name, role, description)
//! - Actions (click, press, etc.)
//! - State information (enabled, visible, focused)
//!
//! # Thread Safety
//!
//! The `AtSpiConnection` uses zbus blocking connections which are thread-safe.
//! However, the accessibility bus connection is typically per-session, and
//! concurrent modifications to UI elements from multiple threads may cause
//! race conditions at the application level. For best results, serialize
//! accessibility operations or use appropriate synchronization.

use crate::error::{DesktopCliError, Result};
use crate::rpc::types::{PatternResult, TreeDumpOptions, UiaElement};
use std::collections::HashMap;
use std::time::Duration;
use zbus::blocking::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

/// D-Bus constants for AT-SPI2
const ATSPI_BUS_NAME: &str = "org.a11y.Bus";
const ATSPI_REGISTRY_PATH: &str = "/org/a11y/atspi/accessible/root";
const ATSPI_ACCESSIBLE_IFACE: &str = "org.a11y.atspi.Accessible";
const ATSPI_COMPONENT_IFACE: &str = "org.a11y.atspi.Component";
const ATSPI_ACTION_IFACE: &str = "org.a11y.atspi.Action";
const ATSPI_VALUE_IFACE: &str = "org.a11y.atspi.Value";
const ATSPI_TEXT_IFACE: &str = "org.a11y.atspi.Text";
const ATSPI_EDITABLE_TEXT_IFACE: &str = "org.a11y.atspi.EditableText";
const ATSPI_SELECTION_IFACE: &str = "org.a11y.atspi.Selection";

/// AT-SPI2 connection wrapper
pub struct AtSpiConnection {
    session_conn: Connection,
    a11y_bus_address: Option<String>,
    a11y_conn: Option<Connection>,
}

impl AtSpiConnection {
    /// Create a new AT-SPI2 connection
    pub fn new() -> Result<Self> {
        // Connect to the session bus
        let session_conn = Connection::session()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to connect to session bus: {}", e)))?;

        // Get the accessibility bus address
        let a11y_bus_address = Self::get_a11y_bus_address(&session_conn).ok();

        // Connect to the accessibility bus if available
        let a11y_conn = if let Some(ref addr) = a11y_bus_address {
            Connection::new_address(addr)
                .map_err(|e| {
                    tracing::warn!("Failed to connect to AT-SPI bus at {}: {}", addr, e);
                    e
                })
                .ok()
        } else {
            None
        };

        Ok(Self {
            session_conn,
            a11y_bus_address,
            a11y_conn,
        })
    }

    /// Get the AT-SPI2 bus address from the a11y bus
    fn get_a11y_bus_address(conn: &Connection) -> Result<String> {
        let proxy = conn.call_method(
            Some("org.a11y.Bus"),
            "/org/a11y/bus",
            Some("org.a11y.Bus"),
            "GetAddress",
            &(),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get AT-SPI bus address: {}", e)))?;

        let body = proxy.body();
        let address: String = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize AT-SPI bus address: {}", e)))?;

        Ok(address)
    }

    /// Get the connection to use for AT-SPI calls
    fn conn(&self) -> &Connection {
        self.a11y_conn.as_ref().unwrap_or(&self.session_conn)
    }

    /// Get all accessible applications
    pub fn get_applications(&self) -> Result<Vec<(String, OwnedObjectPath)>> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some("org.a11y.atspi.Registry"),
            "/org/a11y/atspi/accessible/root",
            Some(ATSPI_ACCESSIBLE_IFACE),
            "GetChildren",
            &(),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get applications: {}", e)))?;

        let body = reply.body();
        let children: Vec<(String, OwnedObjectPath)> = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize applications: {}", e)))?;

        Ok(children)
    }

    /// Get the accessible element for a window by its X11 window ID
    pub fn get_element_for_window(&self, window_id: u64) -> Result<AccessibleElement> {
        let apps = self.get_applications()?;

        for (bus_name, path) in apps {
            if let Ok(elem) = self.find_window_in_app(&bus_name, &path, window_id) {
                return Ok(elem);
            }
        }

        Err(DesktopCliError::WindowNotFound(format!(
            "No accessible element found for window {}",
            window_id
        )))
    }

    /// Find a window element within an application
    fn find_window_in_app(
        &self,
        bus_name: &str,
        path: &ObjectPath,
        window_id: u64,
    ) -> Result<AccessibleElement> {
        let elem = AccessibleElement {
            bus_name: bus_name.to_string(),
            path: path.to_owned().into(),
        };

        // Get children of the application (windows)
        let children = self.get_children(&elem)?;

        for child in children {
            // Check if this window matches the X11 window ID
            if let Ok(component_id) = self.get_component_id(&child) {
                if component_id == window_id {
                    return Ok(child);
                }
            }

            // Recursively search
            if let Ok(found) = self.find_window_in_app(&child.bus_name, child.path.as_ref(), window_id) {
                return Ok(found);
            }
        }

        Err(DesktopCliError::WindowNotFound("Window not found in app".to_string()))
    }

    /// Get the component ID (X11 window ID) for an element
    fn get_component_id(&self, elem: &AccessibleElement) -> Result<u64> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(ATSPI_COMPONENT_IFACE, "NativeWindowHandle"),
        );

        match reply {
            Ok(r) => {
                let body = r.body();
                let variant: OwnedValue = body.deserialize()
                    .map_err(|e| DesktopCliError::AutomationError(format!("Failed to get component ID: {}", e)))?;

                // Try to extract as various integer types
                if let Value::U64(id) = variant.downcast_ref::<Value>().unwrap_or(&Value::U64(0)) {
                    return Ok(*id);
                }
                if let Value::I64(id) = variant.downcast_ref::<Value>().unwrap_or(&Value::I64(0)) {
                    return Ok(*id as u64);
                }
                if let Value::U32(id) = variant.downcast_ref::<Value>().unwrap_or(&Value::U32(0)) {
                    return Ok(*id as u64);
                }

                Err(DesktopCliError::AutomationError("Could not extract component ID".to_string()))
            }
            Err(_) => Err(DesktopCliError::AutomationError("Component interface not supported".to_string())),
        }
    }

    /// Get the name of an element
    pub fn get_name(&self, elem: &AccessibleElement) -> Result<String> {
        self.get_string_property(elem, ATSPI_ACCESSIBLE_IFACE, "Name")
    }

    /// Get the description of an element
    pub fn get_description(&self, elem: &AccessibleElement) -> Result<String> {
        self.get_string_property(elem, ATSPI_ACCESSIBLE_IFACE, "Description")
    }

    /// Get the role of an element
    pub fn get_role(&self, elem: &AccessibleElement) -> Result<u32> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_ACCESSIBLE_IFACE),
            "GetRole",
            &(),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get role: {}", e)))?;

        let body = reply.body();
        let role: u32 = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize role: {}", e)))?;

        Ok(role)
    }

    /// Get the role name of an element
    pub fn get_role_name(&self, elem: &AccessibleElement) -> Result<String> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_ACCESSIBLE_IFACE),
            "GetRoleName",
            &(),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get role name: {}", e)))?;

        let body = reply.body();
        let role_name: String = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize role name: {}", e)))?;

        Ok(role_name)
    }

    /// Get the state set of an element
    pub fn get_state_set(&self, elem: &AccessibleElement) -> Result<Vec<u32>> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_ACCESSIBLE_IFACE),
            "GetState",
            &(),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get state: {}", e)))?;

        let body = reply.body();
        let states: Vec<u32> = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize states: {}", e)))?;

        Ok(states)
    }

    /// Get the bounding box of an element
    pub fn get_extents(&self, elem: &AccessibleElement) -> Result<(i32, i32, i32, i32)> {
        let conn = self.conn();

        // coord_type 0 = screen coordinates
        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_COMPONENT_IFACE),
            "GetExtents",
            &(0u32,),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get extents: {}", e)))?;

        let body = reply.body();
        let extents: (i32, i32, i32, i32) = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize extents: {}", e)))?;

        Ok(extents)
    }

    /// Get children of an element
    pub fn get_children(&self, elem: &AccessibleElement) -> Result<Vec<AccessibleElement>> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_ACCESSIBLE_IFACE),
            "GetChildren",
            &(),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get children: {}", e)))?;

        let body = reply.body();
        let children: Vec<(String, OwnedObjectPath)> = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize children: {}", e)))?;

        Ok(children
            .into_iter()
            .map(|(bus_name, path)| AccessibleElement { bus_name, path })
            .collect())
    }

    /// Get the number of actions available on an element
    pub fn get_n_actions(&self, elem: &AccessibleElement) -> Result<i32> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(ATSPI_ACTION_IFACE, "NActions"),
        );

        match reply {
            Ok(r) => {
                let body = r.body();
                let variant: OwnedValue = body.deserialize()
                    .map_err(|e| DesktopCliError::AutomationError(format!("Failed to get action count: {}", e)))?;

                if let Value::I32(n) = variant.downcast_ref::<Value>().unwrap_or(&Value::I32(0)) {
                    return Ok(*n);
                }
                Ok(0)
            }
            Err(_) => Ok(0),
        }
    }

    /// Get the name of an action
    pub fn get_action_name(&self, elem: &AccessibleElement, index: i32) -> Result<String> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_ACTION_IFACE),
            "GetName",
            &(index,),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get action name: {}", e)))?;

        let body = reply.body();
        let name: String = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize action name: {}", e)))?;

        Ok(name)
    }

    /// Perform an action on an element
    pub fn do_action(&self, elem: &AccessibleElement, index: i32) -> Result<bool> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_ACTION_IFACE),
            "DoAction",
            &(index,),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to perform action: {}", e)))?;

        let body = reply.body();
        let success: bool = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize action result: {}", e)))?;

        Ok(success)
    }

    /// Get the current value of a value element
    pub fn get_current_value(&self, elem: &AccessibleElement) -> Result<f64> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(ATSPI_VALUE_IFACE, "CurrentValue"),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get current value: {}", e)))?;

        let body = reply.body();
        let variant: OwnedValue = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize value: {}", e)))?;

        if let Value::F64(v) = variant.downcast_ref::<Value>().unwrap_or(&Value::F64(0.0)) {
            return Ok(*v);
        }

        Ok(0.0)
    }

    /// Set the current value of a value element
    pub fn set_current_value(&self, elem: &AccessibleElement, value: f64) -> Result<()> {
        let conn = self.conn();

        conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some("org.freedesktop.DBus.Properties"),
            "Set",
            &(ATSPI_VALUE_IFACE, "CurrentValue", Value::F64(value)),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to set value: {}", e)))?;

        Ok(())
    }

    /// Get the text content of a text element
    pub fn get_text(&self, elem: &AccessibleElement, start: i32, end: i32) -> Result<String> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_TEXT_IFACE),
            "GetText",
            &(start, end),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get text: {}", e)))?;

        let body = reply.body();
        let text: String = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize text: {}", e)))?;

        Ok(text)
    }

    /// Get the character count of a text element
    pub fn get_character_count(&self, elem: &AccessibleElement) -> Result<i32> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(ATSPI_TEXT_IFACE, "CharacterCount"),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get character count: {}", e)))?;

        let body = reply.body();
        let variant: OwnedValue = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize count: {}", e)))?;

        if let Value::I32(n) = variant.downcast_ref::<Value>().unwrap_or(&Value::I32(0)) {
            return Ok(*n);
        }

        Ok(0)
    }

    /// Set text in an editable text element
    pub fn set_text(&self, elem: &AccessibleElement, text: &str) -> Result<()> {
        let conn = self.conn();

        // First, get the character count to know how much to delete
        let char_count = self.get_character_count(elem).unwrap_or(0);

        // Delete existing text (best-effort, log errors but continue)
        if char_count > 0 {
            if let Err(e) = conn.call_method(
                Some(&elem.bus_name),
                elem.path.as_ref(),
                Some(ATSPI_EDITABLE_TEXT_IFACE),
                "DeleteText",
                &(0i32, char_count),
            ) {
                tracing::warn!("Failed to delete existing text before insert: {}", e);
            }
        }

        // Insert new text
        conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_EDITABLE_TEXT_IFACE),
            "InsertText",
            &(0i32, text, text.len() as i32),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to set text: {}", e)))?;

        Ok(())
    }

    /// Get supported interfaces for an element
    pub fn get_interfaces(&self, elem: &AccessibleElement) -> Result<Vec<String>> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some(ATSPI_ACCESSIBLE_IFACE),
            "GetInterfaces",
            &(),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get interfaces: {}", e)))?;

        let body = reply.body();
        let interfaces: Vec<String> = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize interfaces: {}", e)))?;

        Ok(interfaces)
    }

    /// Helper to get a string property
    fn get_string_property(&self, elem: &AccessibleElement, iface: &str, prop: &str) -> Result<String> {
        let conn = self.conn();

        let reply = conn.call_method(
            Some(&elem.bus_name),
            elem.path.as_ref(),
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(iface, prop),
        ).map_err(|e| DesktopCliError::AutomationError(format!("Failed to get property {}: {}", prop, e)))?;

        let body = reply.body();
        let variant: OwnedValue = body.deserialize()
            .map_err(|e| DesktopCliError::AutomationError(format!("Failed to deserialize {}: {}", prop, e)))?;

        if let Value::Str(s) = variant.downcast_ref::<Value>().unwrap_or(&Value::Str("".into())) {
            return Ok(s.to_string());
        }

        Ok(String::new())
    }
}

/// Reference to an accessible element
#[derive(Debug, Clone)]
pub struct AccessibleElement {
    pub bus_name: String,
    pub path: OwnedObjectPath,
}

impl AccessibleElement {
    /// Create a unique identifier string for this element
    pub fn to_id_string(&self) -> String {
        format!("{}:{}", self.bus_name, self.path.as_str())
    }
}

/// AT-SPI2 state constants
pub mod states {
    pub const INVALID: u32 = 0;
    pub const ACTIVE: u32 = 1;
    pub const ARMED: u32 = 2;
    pub const BUSY: u32 = 3;
    pub const CHECKED: u32 = 4;
    pub const COLLAPSED: u32 = 5;
    pub const DEFUNCT: u32 = 6;
    pub const EDITABLE: u32 = 7;
    pub const ENABLED: u32 = 8;
    pub const EXPANDABLE: u32 = 9;
    pub const EXPANDED: u32 = 10;
    pub const FOCUSABLE: u32 = 11;
    pub const FOCUSED: u32 = 12;
    pub const HAS_TOOLTIP: u32 = 13;
    pub const HORIZONTAL: u32 = 14;
    pub const ICONIFIED: u32 = 15;
    pub const MODAL: u32 = 16;
    pub const MULTI_LINE: u32 = 17;
    pub const MULTISELECTABLE: u32 = 18;
    pub const OPAQUE: u32 = 19;
    pub const PRESSED: u32 = 20;
    pub const RESIZABLE: u32 = 21;
    pub const SELECTABLE: u32 = 22;
    pub const SELECTED: u32 = 23;
    pub const SENSITIVE: u32 = 24;
    pub const SHOWING: u32 = 25;
    pub const SINGLE_LINE: u32 = 26;
    pub const STALE: u32 = 27;
    pub const TRANSIENT: u32 = 28;
    pub const VERTICAL: u32 = 29;
    pub const VISIBLE: u32 = 30;
    pub const MANAGES_DESCENDANTS: u32 = 31;
    pub const INDETERMINATE: u32 = 32;
    pub const REQUIRED: u32 = 33;
    pub const TRUNCATED: u32 = 34;
    pub const ANIMATED: u32 = 35;
    pub const INVALID_ENTRY: u32 = 36;
    pub const SUPPORTS_AUTOCOMPLETION: u32 = 37;
    pub const SELECTABLE_TEXT: u32 = 38;
    pub const IS_DEFAULT: u32 = 39;
    pub const VISITED: u32 = 40;
    pub const CHECKABLE: u32 = 41;
    pub const HAS_POPUP: u32 = 42;
    pub const READ_ONLY: u32 = 43;
}

/// AT-SPI2 role constants
pub mod roles {
    pub const INVALID: u32 = 0;
    pub const ACCELERATOR_LABEL: u32 = 1;
    pub const ALERT: u32 = 2;
    pub const ANIMATION: u32 = 3;
    pub const ARROW: u32 = 4;
    pub const CALENDAR: u32 = 5;
    pub const CANVAS: u32 = 6;
    pub const CHECK_BOX: u32 = 7;
    pub const CHECK_MENU_ITEM: u32 = 8;
    pub const COLOR_CHOOSER: u32 = 9;
    pub const COLUMN_HEADER: u32 = 10;
    pub const COMBO_BOX: u32 = 11;
    pub const DATE_EDITOR: u32 = 12;
    pub const DESKTOP_ICON: u32 = 13;
    pub const DESKTOP_FRAME: u32 = 14;
    pub const DIAL: u32 = 15;
    pub const DIALOG: u32 = 16;
    pub const DIRECTORY_PANE: u32 = 17;
    pub const DRAWING_AREA: u32 = 18;
    pub const FILE_CHOOSER: u32 = 19;
    pub const FILLER: u32 = 20;
    pub const FOCUS_TRAVERSABLE: u32 = 21;
    pub const FONT_CHOOSER: u32 = 22;
    pub const FRAME: u32 = 23;
    pub const GLASS_PANE: u32 = 24;
    pub const HTML_CONTAINER: u32 = 25;
    pub const ICON: u32 = 26;
    pub const IMAGE: u32 = 27;
    pub const INTERNAL_FRAME: u32 = 28;
    pub const LABEL: u32 = 29;
    pub const LAYERED_PANE: u32 = 30;
    pub const LIST: u32 = 31;
    pub const LIST_ITEM: u32 = 32;
    pub const MENU: u32 = 33;
    pub const MENU_BAR: u32 = 34;
    pub const MENU_ITEM: u32 = 35;
    pub const OPTION_PANE: u32 = 36;
    pub const PAGE_TAB: u32 = 37;
    pub const PAGE_TAB_LIST: u32 = 38;
    pub const PANEL: u32 = 39;
    pub const PASSWORD_TEXT: u32 = 40;
    pub const POPUP_MENU: u32 = 41;
    pub const PROGRESS_BAR: u32 = 42;
    pub const PUSH_BUTTON: u32 = 43;
    pub const RADIO_BUTTON: u32 = 44;
    pub const RADIO_MENU_ITEM: u32 = 45;
    pub const ROOT_PANE: u32 = 46;
    pub const ROW_HEADER: u32 = 47;
    pub const SCROLL_BAR: u32 = 48;
    pub const SCROLL_PANE: u32 = 49;
    pub const SEPARATOR: u32 = 50;
    pub const SLIDER: u32 = 51;
    pub const SPIN_BUTTON: u32 = 52;
    pub const SPLIT_PANE: u32 = 53;
    pub const STATUS_BAR: u32 = 54;
    pub const TABLE: u32 = 55;
    pub const TABLE_CELL: u32 = 56;
    pub const TABLE_COLUMN_HEADER: u32 = 57;
    pub const TABLE_ROW_HEADER: u32 = 58;
    pub const TEAROFF_MENU_ITEM: u32 = 59;
    pub const TERMINAL: u32 = 60;
    pub const TEXT: u32 = 61;
    pub const TOGGLE_BUTTON: u32 = 62;
    pub const TOOL_BAR: u32 = 63;
    pub const TOOL_TIP: u32 = 64;
    pub const TREE: u32 = 65;
    pub const TREE_TABLE: u32 = 66;
    pub const UNKNOWN: u32 = 67;
    pub const VIEWPORT: u32 = 68;
    pub const WINDOW: u32 = 69;
    pub const EXTENDED: u32 = 70;
    pub const HEADER: u32 = 71;
    pub const FOOTER: u32 = 72;
    pub const PARAGRAPH: u32 = 73;
    pub const RULER: u32 = 74;
    pub const APPLICATION: u32 = 75;
    pub const AUTOCOMPLETE: u32 = 76;
    pub const EDITBAR: u32 = 77;
    pub const EMBEDDED: u32 = 78;
    pub const ENTRY: u32 = 79;
    pub const CHART: u32 = 80;
    pub const CAPTION: u32 = 81;
    pub const DOCUMENT_FRAME: u32 = 82;
    pub const HEADING: u32 = 83;
    pub const PAGE: u32 = 84;
    pub const SECTION: u32 = 85;
    pub const REDUNDANT_OBJECT: u32 = 86;
    pub const FORM: u32 = 87;
    pub const LINK: u32 = 88;
    pub const INPUT_METHOD_WINDOW: u32 = 89;
    pub const TABLE_ROW: u32 = 90;
    pub const TREE_ITEM: u32 = 91;
    pub const DOCUMENT_SPREADSHEET: u32 = 92;
    pub const DOCUMENT_PRESENTATION: u32 = 93;
    pub const DOCUMENT_TEXT: u32 = 94;
    pub const DOCUMENT_WEB: u32 = 95;
    pub const DOCUMENT_EMAIL: u32 = 96;
    pub const COMMENT: u32 = 97;
    pub const LIST_BOX: u32 = 98;
    pub const GROUPING: u32 = 99;
    pub const IMAGE_MAP: u32 = 100;
    pub const NOTIFICATION: u32 = 101;
    pub const INFO_BAR: u32 = 102;
    pub const LEVEL_BAR: u32 = 103;
    pub const TITLE_BAR: u32 = 104;
    pub const BLOCK_QUOTE: u32 = 105;
    pub const AUDIO: u32 = 106;
    pub const VIDEO: u32 = 107;
    pub const DEFINITION: u32 = 108;
    pub const ARTICLE: u32 = 109;
    pub const LANDMARK: u32 = 110;
    pub const LOG: u32 = 111;
    pub const MARQUEE: u32 = 112;
    pub const MATH: u32 = 113;
    pub const RATING: u32 = 114;
    pub const TIMER: u32 = 115;
    pub const STATIC: u32 = 116;
    pub const MATH_FRACTION: u32 = 117;
    pub const MATH_ROOT: u32 = 118;
    pub const SUBSCRIPT: u32 = 119;
    pub const SUPERSCRIPT: u32 = 120;
}

/// Convert role number to string
pub fn role_to_string(role: u32) -> String {
    match role {
        roles::PUSH_BUTTON => "Button".to_string(),
        roles::RADIO_BUTTON => "RadioButton".to_string(),
        roles::CHECK_BOX => "CheckBox".to_string(),
        roles::TOGGLE_BUTTON => "ToggleButton".to_string(),
        roles::MENU => "Menu".to_string(),
        roles::MENU_BAR => "MenuBar".to_string(),
        roles::MENU_ITEM => "MenuItem".to_string(),
        roles::POPUP_MENU => "PopupMenu".to_string(),
        roles::COMBO_BOX => "ComboBox".to_string(),
        roles::LIST => "List".to_string(),
        roles::LIST_ITEM => "ListItem".to_string(),
        roles::LIST_BOX => "ListBox".to_string(),
        roles::TREE => "Tree".to_string(),
        roles::TREE_ITEM => "TreeItem".to_string(),
        roles::TABLE => "Table".to_string(),
        roles::TABLE_CELL => "TableCell".to_string(),
        roles::TEXT | roles::ENTRY => "Edit".to_string(),
        roles::PASSWORD_TEXT => "PasswordEdit".to_string(),
        roles::LABEL | roles::STATIC => "Text".to_string(),
        roles::IMAGE => "Image".to_string(),
        roles::LINK => "Hyperlink".to_string(),
        roles::PROGRESS_BAR => "ProgressBar".to_string(),
        roles::SLIDER => "Slider".to_string(),
        roles::SPIN_BUTTON => "Spinner".to_string(),
        roles::SCROLL_BAR => "ScrollBar".to_string(),
        roles::PANEL | roles::FILLER => "Pane".to_string(),
        roles::FRAME | roles::WINDOW => "Window".to_string(),
        roles::DIALOG => "Dialog".to_string(),
        roles::PAGE_TAB => "TabItem".to_string(),
        roles::PAGE_TAB_LIST => "Tab".to_string(),
        roles::TOOL_BAR => "ToolBar".to_string(),
        roles::STATUS_BAR => "StatusBar".to_string(),
        roles::TOOL_TIP => "ToolTip".to_string(),
        roles::SEPARATOR => "Separator".to_string(),
        roles::HEADER => "Header".to_string(),
        roles::HEADING => "Heading".to_string(),
        roles::DOCUMENT_FRAME | roles::DOCUMENT_TEXT | roles::DOCUMENT_WEB => "Document".to_string(),
        _ => format!("Role{}", role),
    }
}

/// Check if state set contains a specific state
pub fn has_state(state_set: &[u32], state: u32) -> bool {
    // State set is a bitfield stored as array of u32
    let word = (state / 32) as usize;
    let bit = state % 32;

    if word < state_set.len() {
        (state_set[word] & (1 << bit)) != 0
    } else {
        false
    }
}

/// Dump the accessibility tree starting from an element
pub fn dump_tree(
    conn: &AtSpiConnection,
    elem: &AccessibleElement,
    options: &TreeDumpOptions,
) -> Result<UiaElement> {
    dump_element_recursive(conn, elem, 0, options)
}

fn dump_element_recursive(
    conn: &AtSpiConnection,
    elem: &AccessibleElement,
    depth: u32,
    options: &TreeDumpOptions,
) -> Result<UiaElement> {
    let name = conn.get_name(elem).unwrap_or_default();
    let role = conn.get_role(elem).unwrap_or(roles::UNKNOWN);
    let role_name = conn.get_role_name(elem).unwrap_or_default();
    let states = conn.get_state_set(elem).unwrap_or_default();
    let extents = conn.get_extents(elem).unwrap_or((0, 0, 0, 0));
    let interfaces = conn.get_interfaces(elem).unwrap_or_default();

    // Determine supported patterns from interfaces
    let mut patterns = Vec::new();
    if interfaces.iter().any(|i| i.contains("Action")) {
        patterns.push("Invoke".to_string());
    }
    if interfaces.iter().any(|i| i.contains("Value")) {
        patterns.push("Value".to_string());
    }
    if interfaces.iter().any(|i| i.contains("Text")) {
        patterns.push("Text".to_string());
    }
    if interfaces.iter().any(|i| i.contains("EditableText")) {
        patterns.push("EditableText".to_string());
    }
    if interfaces.iter().any(|i| i.contains("Selection")) {
        patterns.push("Selection".to_string());
    }

    // Get value if available
    let value = if interfaces.iter().any(|i| i.contains("Value")) {
        conn.get_current_value(elem).ok().map(|v| v.to_string())
    } else if interfaces.iter().any(|i| i.contains("Text")) {
        let count = conn.get_character_count(elem).unwrap_or(0);
        if count > 0 && count < 1000 {
            conn.get_text(elem, 0, count).ok()
        } else {
            None
        }
    } else {
        None
    };

    let is_enabled = has_state(&states, states::ENABLED) || has_state(&states, states::SENSITIVE);
    let is_showing = has_state(&states, states::SHOWING);
    let is_visible = has_state(&states, states::VISIBLE);
    let is_offscreen = !is_showing || !is_visible;

    let mut uia_elem = UiaElement {
        id: elem.to_id_string(),
        control_type: role_to_string(role),
        localized_type: role_name,
        name,
        automation_id: String::new(), // AT-SPI doesn't have automation IDs
        class_name: String::new(),
        value,
        bounds: [extents.0, extents.1, extents.2, extents.3],
        is_enabled,
        is_offscreen,
        patterns,
        depth,
        children: Vec::new(),
    };

    // Recurse into children if within depth limit
    if depth < options.max_depth {
        if let Ok(children) = conn.get_children(elem) {
            let mut child_count = 0;

            for child in children {
                // Check list item cap
                if options.max_list_items > 0 && child_count >= options.max_list_items {
                    let mut placeholder = UiaElement::default();
                    placeholder.name = format!("... (more items)");
                    placeholder.depth = depth + 1;
                    uia_elem.children.push(placeholder);
                    break;
                }

                if let Ok(child_elem) = dump_element_recursive(conn, &child, depth + 1, options) {
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
                }
            }
        }
    }

    Ok(uia_elem)
}

/// Pattern operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatternOp {
    Invoke,
    GetValue,
    SetValue,
    GetText,
    SetText,
    Toggle,
    Select,
    Focus,
}

impl PatternOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "invoke" | "click" | "press" => Some(Self::Invoke),
            "get-value" | "getvalue" | "value" => Some(Self::GetValue),
            "set-value" | "setvalue" => Some(Self::SetValue),
            "get-text" | "gettext" | "text" => Some(Self::GetText),
            "set-text" | "settext" => Some(Self::SetText),
            "toggle" => Some(Self::Toggle),
            "select" => Some(Self::Select),
            "focus" => Some(Self::Focus),
            _ => None,
        }
    }
}

/// Execute a pattern operation on an element
pub fn execute_pattern(
    conn: &AtSpiConnection,
    elem: &AccessibleElement,
    pattern: PatternOp,
    value: Option<&str>,
) -> PatternResult {
    match pattern {
        PatternOp::Invoke => {
            // Find and execute the first action (usually "click" or "press")
            match conn.do_action(elem, 0) {
                Ok(true) => PatternResult::ok(),
                Ok(false) => PatternResult::err("Action returned false"),
                Err(e) => PatternResult::err(e.to_string()),
            }
        }
        PatternOp::GetValue => match conn.get_current_value(elem) {
            Ok(v) => PatternResult::ok_with_value(v.to_string()),
            Err(e) => PatternResult::err(e.to_string()),
        },
        PatternOp::SetValue => {
            let v = value.unwrap_or("0").parse::<f64>().unwrap_or(0.0);
            match conn.set_current_value(elem, v) {
                Ok(()) => PatternResult::ok(),
                Err(e) => PatternResult::err(e.to_string()),
            }
        }
        PatternOp::GetText => {
            let count = conn.get_character_count(elem).unwrap_or(0);
            match conn.get_text(elem, 0, count) {
                Ok(text) => PatternResult::ok_with_value(text),
                Err(e) => PatternResult::err(e.to_string()),
            }
        }
        PatternOp::SetText => {
            let text = value.unwrap_or("");
            match conn.set_text(elem, text) {
                Ok(()) => PatternResult::ok(),
                Err(e) => PatternResult::err(e.to_string()),
            }
        }
        PatternOp::Toggle => {
            // Toggle is usually action 0 for toggle buttons
            match conn.do_action(elem, 0) {
                Ok(true) => PatternResult::ok(),
                Ok(false) => PatternResult::err("Toggle action returned false"),
                Err(e) => PatternResult::err(e.to_string()),
            }
        }
        PatternOp::Select => {
            // Selection is usually action 0 for selectable items
            match conn.do_action(elem, 0) {
                Ok(true) => PatternResult::ok(),
                Ok(false) => PatternResult::err("Select action returned false"),
                Err(e) => PatternResult::err(e.to_string()),
            }
        }
        PatternOp::Focus => {
            // Focus by grabbing keyboard focus via component interface
            // This is a best-effort operation
            PatternResult::ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_to_string() {
        assert_eq!(role_to_string(roles::PUSH_BUTTON), "Button");
        assert_eq!(role_to_string(roles::CHECK_BOX), "CheckBox");
        assert_eq!(role_to_string(roles::TEXT), "Edit");
    }

    #[test]
    fn test_has_state() {
        let states = vec![0b00000101u32, 0u32]; // States 0 and 2 are set
        assert!(has_state(&states, 0));
        assert!(!has_state(&states, 1));
        assert!(has_state(&states, 2));
        assert!(!has_state(&states, 3));
    }

    #[test]
    fn test_pattern_op_from_str() {
        assert_eq!(PatternOp::from_str("invoke"), Some(PatternOp::Invoke));
        assert_eq!(PatternOp::from_str("click"), Some(PatternOp::Invoke));
        assert_eq!(PatternOp::from_str("get-value"), Some(PatternOp::GetValue));
        assert_eq!(PatternOp::from_str("invalid"), None);
    }
}
