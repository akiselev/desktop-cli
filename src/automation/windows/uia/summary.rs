//! Summary view and compact output formats optimized for LLM consumption
//!
//! This module provides heuristics to maximize signal-to-noise ratio when
//! presenting UI Automation data to language models.

use crate::rpc::types::UiaElement;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compact element representation optimized for LLM context windows
/// Uses ~70% less tokens than full UiaElement while preserving actionable info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactElement {
    /// Short reference ID for use in selectors (e.g., "b1", "e3", "t5")
    pub ref_id: String,
    /// Semantic role: button, input, text, menu, list, tree, tab, etc.
    pub role: String,
    /// Display label (name or inferred from context)
    pub label: String,
    /// Primary action available (click, type, toggle, expand, select)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Current value for inputs/toggles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// State flags as compact string: "disabled", "checked", "expanded"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Breadcrumb path showing UI hierarchy (e.g., "Menu > File > Recent")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Bounding box as [x, y, w, h] - only included if --include-bounds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[i32; 4]>,
    /// Nested children in compact form
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<CompactElement>,
}

/// Summary of the current UI state - returned after every interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSummary {
    /// Active window title
    pub window: String,
    /// Currently focused element (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<CompactElement>,
    /// Top-level actionable elements (buttons, inputs, etc.)
    pub actions: Vec<CompactElement>,
    /// Navigation elements (menus, tabs, tree items)
    pub navigation: Vec<CompactElement>,
    /// Content areas with notable elements
    pub content: Vec<CompactElement>,
    /// Status/info areas (status bars, tooltips)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub status: Vec<CompactElement>,
    /// Element counts for context
    pub stats: UiStats,
}

/// Statistics about the UI tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStats {
    pub total_elements: u32,
    pub visible_elements: u32,
    pub actionable_elements: u32,
    pub pruned_elements: u32,
}

/// Options for summary generation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SummaryOptions {
    /// Include bounding boxes in output
    pub include_bounds: bool,
    /// Include full paths for all elements
    pub include_paths: bool,
    /// Focus on a specific region [x, y, w, h]
    pub focus_region: Option<[i32; 4]>,
    /// Maximum depth for action/nav elements
    pub max_depth: u32,
    /// Minimum element size to include (filter tiny elements)
    pub min_size: u32,
    /// Only include elements matching these roles
    pub role_filter: Option<Vec<String>>,
}

/// Semantic roles for UI elements (more meaningful than raw ControlTypes)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticRole {
    // Primary actions
    Button,
    Link,
    MenuItem,
    // Input
    TextInput,
    Checkbox,
    RadioButton,
    Slider,
    Dropdown,
    // Navigation
    Tab,
    TreeItem,
    ListItem,
    // Containers
    Menu,
    Toolbar,
    Panel,
    Group,
    Dialog,
    // Content
    Text,
    Image,
    Table,
    // Special
    ScrollBar,
    Tooltip,
    StatusBar,
    Unknown,
}

impl SemanticRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Link => "link",
            Self::MenuItem => "menuitem",
            Self::TextInput => "input",
            Self::Checkbox => "checkbox",
            Self::RadioButton => "radio",
            Self::Slider => "slider",
            Self::Dropdown => "dropdown",
            Self::Tab => "tab",
            Self::TreeItem => "treeitem",
            Self::ListItem => "listitem",
            Self::Menu => "menu",
            Self::Toolbar => "toolbar",
            Self::Panel => "panel",
            Self::Group => "group",
            Self::Dialog => "dialog",
            Self::Text => "text",
            Self::Image => "image",
            Self::Table => "table",
            Self::ScrollBar => "scrollbar",
            Self::Tooltip => "tooltip",
            Self::StatusBar => "statusbar",
            Self::Unknown => "element",
        }
    }

    /// Whether this role typically represents an actionable element
    pub fn is_actionable(&self) -> bool {
        matches!(
            self,
            Self::Button
                | Self::Link
                | Self::MenuItem
                | Self::TextInput
                | Self::Checkbox
                | Self::RadioButton
                | Self::Slider
                | Self::Dropdown
                | Self::Tab
                | Self::TreeItem
                | Self::ListItem
        )
    }

    /// Whether this role is a navigation container
    pub fn is_navigation(&self) -> bool {
        matches!(self, Self::Menu | Self::Toolbar | Self::Tab)
    }
}

/// Infer semantic role from UIA control type and patterns
pub fn infer_role(elem: &UiaElement) -> SemanticRole {
    let ct = elem.control_type.to_lowercase();
    let patterns = &elem.patterns;

    match ct.as_str() {
        "button" => {
            if patterns.contains(&"Toggle".to_string()) {
                if elem.name.to_lowercase().contains("check")
                    || elem.class_name.to_lowercase().contains("checkbox")
                {
                    SemanticRole::Checkbox
                } else {
                    SemanticRole::Button
                }
            } else {
                SemanticRole::Button
            }
        }
        "checkbox" => SemanticRole::Checkbox,
        "radiobutton" => SemanticRole::RadioButton,
        "edit" | "document" => SemanticRole::TextInput,
        "combobox" => SemanticRole::Dropdown,
        "slider" | "spinner" => SemanticRole::Slider,
        "hyperlink" => SemanticRole::Link,
        "menuitem" => SemanticRole::MenuItem,
        "menu" | "menubar" => SemanticRole::Menu,
        "tabitem" => SemanticRole::Tab,
        "tab" => SemanticRole::Panel, // Tab container
        "treeitem" => SemanticRole::TreeItem,
        "listitem" | "dataitem" => SemanticRole::ListItem,
        "list" => SemanticRole::Panel,
        "tree" => SemanticRole::Panel,
        "toolbar" => SemanticRole::Toolbar,
        "pane" | "window" | "custom" => {
            // Infer from patterns
            if patterns.contains(&"Invoke".to_string()) {
                SemanticRole::Button
            } else if patterns.contains(&"Value".to_string()) {
                SemanticRole::TextInput
            } else if patterns.contains(&"ExpandCollapse".to_string()) {
                SemanticRole::TreeItem
            } else {
                SemanticRole::Panel
            }
        }
        "group" => SemanticRole::Group,
        "text" => SemanticRole::Text,
        "image" => SemanticRole::Image,
        "table" | "datagrid" => SemanticRole::Table,
        "scrollbar" => SemanticRole::ScrollBar,
        "tooltip" => SemanticRole::Tooltip,
        "statusbar" => SemanticRole::StatusBar,
        "titlebar" | "header" | "headeritem" => SemanticRole::Panel,
        _ => SemanticRole::Unknown,
    }
}

/// Infer the primary action available for an element
pub fn infer_action(elem: &UiaElement) -> Option<String> {
    let patterns = &elem.patterns;

    if patterns.contains(&"Invoke".to_string()) {
        Some("click".to_string())
    } else if patterns.contains(&"Toggle".to_string()) {
        Some("toggle".to_string())
    } else if patterns.contains(&"Value".to_string()) {
        Some("type".to_string())
    } else if patterns.contains(&"ExpandCollapse".to_string()) {
        Some("expand".to_string())
    } else if patterns.contains(&"SelectionItem".to_string()) {
        Some("select".to_string())
    } else if patterns.contains(&"RangeValue".to_string()) {
        Some("slide".to_string())
    } else if patterns.contains(&"Scroll".to_string()) {
        Some("scroll".to_string())
    } else {
        None
    }
}

/// Build state string from element properties
pub fn build_state_string(elem: &UiaElement) -> Option<String> {
    let mut states = Vec::new();

    if !elem.is_enabled {
        states.push("disabled");
    }
    if elem.is_offscreen {
        states.push("offscreen");
    }

    // Check toggle state from value
    if let Some(ref val) = elem.value {
        let v = val.to_lowercase();
        if v == "on" || v == "1" || v == "true" {
            states.push("checked");
        }
    }

    // Check if expanded (from ExpandCollapse pattern - would need actual state)
    // For now, we can infer from patterns and name
    if elem.patterns.contains(&"ExpandCollapse".to_string()) {
        // Could be expanded or collapsed - would need actual state query
    }

    if states.is_empty() {
        None
    } else {
        Some(states.join(", "))
    }
}

/// Reference ID generator for compact elements
pub struct RefIdGenerator {
    counters: HashMap<char, u32>,
}

impl RefIdGenerator {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Generate a short reference ID based on role
    pub fn next(&mut self, role: SemanticRole) -> String {
        let prefix = match role {
            SemanticRole::Button | SemanticRole::Link => 'b',
            SemanticRole::TextInput => 'i',
            SemanticRole::Checkbox | SemanticRole::RadioButton => 'c',
            SemanticRole::MenuItem => 'm',
            SemanticRole::Tab => 't',
            SemanticRole::TreeItem | SemanticRole::ListItem => 'l',
            SemanticRole::Dropdown => 'd',
            SemanticRole::Slider => 's',
            SemanticRole::Panel | SemanticRole::Group => 'p',
            SemanticRole::Menu | SemanticRole::Toolbar => 'n',
            SemanticRole::Text => 'x',
            SemanticRole::Table => 'g',
            _ => 'e',
        };

        let count = self.counters.entry(prefix).or_insert(0);
        *count += 1;
        format!("{}{}", prefix, count)
    }
}

impl Default for RefIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Heuristics for filtering noise from UI trees
pub struct FilterHeuristics;

impl FilterHeuristics {
    /// Check if element is likely just decorative/structural noise
    pub fn is_noise(elem: &UiaElement) -> bool {
        // Empty elements with no children or patterns
        if elem.name.is_empty()
            && elem.patterns.is_empty()
            && elem.children.is_empty()
            && elem.automation_id.is_empty()
        {
            return true;
        }

        // Very small elements (likely separators, borders)
        let [_, _, w, h] = elem.bounds;
        if w > 0 && h > 0 && w < 5 && h < 5 {
            return true;
        }

        // Scrollbar internals
        let ct = elem.control_type.to_lowercase();
        if ct == "thumb" || ct == "scrollbar" {
            return true;
        }

        // Common decorative class names
        let class = elem.class_name.to_lowercase();
        if class.contains("separator")
            || class.contains("border")
            || class.contains("spacer")
            || class.contains("grip")
        {
            return true;
        }

        false
    }

    /// Check if element is in a repeated list that should be collapsed
    pub fn is_repeated_item(elem: &UiaElement, siblings: &[UiaElement]) -> bool {
        // If there are many siblings with the same control type, it's a list
        if siblings.len() > 5 {
            let same_type_count = siblings
                .iter()
                .filter(|s| s.control_type == elem.control_type)
                .count();
            return same_type_count > 5;
        }
        false
    }

    /// Check if element is within a focus region
    pub fn in_region(elem: &UiaElement, region: &[i32; 4]) -> bool {
        let [rx, ry, rw, rh] = region;
        let [ex, ey, ew, eh] = elem.bounds;

        // Check if element bounds overlap with region
        let elem_right = ex + ew;
        let elem_bottom = ey + eh;
        let region_right = rx + rw;
        let region_bottom = ry + rh;

        ex < region_right && elem_right > *rx && ey < region_bottom && elem_bottom > *ry
    }

    /// Compute importance score for an element (higher = more important)
    pub fn importance_score(elem: &UiaElement) -> u32 {
        let mut score = 0u32;

        // Has a meaningful name
        if !elem.name.is_empty() && elem.name.len() > 1 {
            score += 10;
        }

        // Has actionable patterns
        if elem.patterns.contains(&"Invoke".to_string()) {
            score += 20;
        }
        if elem.patterns.contains(&"Value".to_string()) {
            score += 15;
        }
        if elem.patterns.contains(&"Toggle".to_string()) {
            score += 15;
        }
        if elem.patterns.contains(&"SelectionItem".to_string()) {
            score += 10;
        }

        // Has automation ID (developer intended this to be accessible)
        if !elem.automation_id.is_empty() {
            score += 5;
        }

        // Is enabled
        if elem.is_enabled {
            score += 5;
        }

        // Size-based score (larger elements tend to be more important)
        let [_, _, w, h] = elem.bounds;
        if w > 100 || h > 50 {
            score += 5;
        }

        score
    }
}

/// Convert a UiaElement tree to a compact summary
pub fn to_compact(
    elem: &UiaElement,
    options: &SummaryOptions,
    id_gen: &mut RefIdGenerator,
    parent_path: Option<&str>,
) -> Option<CompactElement> {
    // Check region filter
    if let Some(ref region) = options.focus_region {
        if !FilterHeuristics::in_region(elem, region) {
            return None;
        }
    }

    // Check depth
    if elem.depth > options.max_depth && options.max_depth > 0 {
        return None;
    }

    // Check size filter
    if options.min_size > 0 {
        let [_, _, w, h] = elem.bounds;
        if (w as u32) < options.min_size && (h as u32) < options.min_size {
            return None;
        }
    }

    // Skip noise elements
    if FilterHeuristics::is_noise(elem) {
        return None;
    }

    let role = infer_role(elem);

    // Check role filter
    if let Some(ref roles) = options.role_filter {
        if !roles.contains(&role.as_str().to_string()) {
            return None;
        }
    }

    let ref_id = id_gen.next(role);

    // Build label - prefer name, fall back to automation_id
    let label = if !elem.name.is_empty() {
        elem.name.clone()
    } else if !elem.automation_id.is_empty() {
        elem.automation_id.clone()
    } else if !elem.class_name.is_empty() {
        elem.class_name.clone()
    } else {
        role.as_str().to_string()
    };

    // Build path
    let path = if options.include_paths || parent_path.is_some() {
        if let Some(pp) = parent_path {
            if !elem.name.is_empty() {
                Some(format!("{} > {}", pp, elem.name))
            } else {
                Some(pp.to_string())
            }
        } else if !elem.name.is_empty() {
            Some(elem.name.clone())
        } else {
            None
        }
    } else {
        None
    };

    // Recursively process children
    let children: Vec<CompactElement> = elem
        .children
        .iter()
        .filter_map(|child| to_compact(child, options, id_gen, path.as_deref()))
        .collect();

    Some(CompactElement {
        ref_id,
        role: role.as_str().to_string(),
        label,
        action: infer_action(elem),
        value: elem.value.clone(),
        state: build_state_string(elem),
        path,
        bounds: if options.include_bounds {
            Some(elem.bounds)
        } else {
            None
        },
        children,
    })
}

/// Generate a full UI summary from a UiaElement tree
pub fn generate_summary(
    root: &UiaElement,
    window_title: &str,
    options: &SummaryOptions,
) -> UiSummary {
    let mut id_gen = RefIdGenerator::new();
    let mut actions = Vec::new();
    let mut navigation = Vec::new();
    let mut content = Vec::new();
    let mut status = Vec::new();

    // Stats tracking
    let mut total_elements = 0u32;
    let mut visible_elements = 0u32;
    let mut actionable_elements = 0u32;
    let mut pruned_elements = 0u32;

    // Categorize elements
    fn categorize_recursive(
        elem: &UiaElement,
        options: &SummaryOptions,
        id_gen: &mut RefIdGenerator,
        actions: &mut Vec<CompactElement>,
        navigation: &mut Vec<CompactElement>,
        content: &mut Vec<CompactElement>,
        status: &mut Vec<CompactElement>,
        total: &mut u32,
        visible: &mut u32,
        actionable: &mut u32,
        pruned: &mut u32,
        parent_path: Option<&str>,
    ) {
        *total += 1;

        if elem.is_offscreen {
            *pruned += 1;
            return;
        }
        *visible += 1;

        let role = infer_role(elem);
        if role.is_actionable() {
            *actionable += 1;
        }

        // Skip noise
        if FilterHeuristics::is_noise(elem) {
            *pruned += 1;
            return;
        }

        // Build path for children
        let path = if !elem.name.is_empty() {
            if let Some(pp) = parent_path {
                Some(format!("{} > {}", pp, elem.name))
            } else {
                Some(elem.name.clone())
            }
        } else {
            parent_path.map(|s| s.to_string())
        };

        // Categorize based on role
        match role {
            SemanticRole::Button
            | SemanticRole::Link
            | SemanticRole::TextInput
            | SemanticRole::Checkbox
            | SemanticRole::RadioButton
            | SemanticRole::Dropdown
            | SemanticRole::Slider => {
                if let Some(compact) = to_compact(elem, options, id_gen, parent_path) {
                    actions.push(compact);
                }
            }
            SemanticRole::Menu
            | SemanticRole::Toolbar
            | SemanticRole::Tab
            | SemanticRole::MenuItem
            | SemanticRole::TreeItem => {
                if let Some(compact) = to_compact(elem, options, id_gen, parent_path) {
                    navigation.push(compact);
                }
            }
            SemanticRole::StatusBar | SemanticRole::Tooltip => {
                if let Some(compact) = to_compact(elem, options, id_gen, parent_path) {
                    status.push(compact);
                }
            }
            _ => {
                // For containers, recurse into children
                for child in &elem.children {
                    categorize_recursive(
                        child,
                        options,
                        id_gen,
                        actions,
                        navigation,
                        content,
                        status,
                        total,
                        visible,
                        actionable,
                        pruned,
                        path.as_deref(),
                    );
                }
            }
        }
    }

    categorize_recursive(
        root,
        options,
        &mut id_gen,
        &mut actions,
        &mut navigation,
        &mut content,
        &mut status,
        &mut total_elements,
        &mut visible_elements,
        &mut actionable_elements,
        &mut pruned_elements,
        None,
    );

    UiSummary {
        window: window_title.to_string(),
        focus: None, // Would need focused element query
        actions,
        navigation,
        content,
        status,
        stats: UiStats {
            total_elements,
            visible_elements,
            actionable_elements,
            pruned_elements,
        },
    }
}

/// Format a UiSummary as a compact text representation (even smaller than JSON)
pub fn format_text_summary(summary: &UiSummary) -> String {
    let mut lines = Vec::new();

    lines.push(format!("# {}", summary.window));
    lines.push(String::new());

    if !summary.actions.is_empty() {
        lines.push("## Actions".to_string());
        for elem in &summary.actions {
            lines.push(format_compact_element(elem, 0));
        }
        lines.push(String::new());
    }

    if !summary.navigation.is_empty() {
        lines.push("## Navigation".to_string());
        for elem in &summary.navigation {
            lines.push(format_compact_element(elem, 0));
        }
        lines.push(String::new());
    }

    if !summary.status.is_empty() {
        lines.push("## Status".to_string());
        for elem in &summary.status {
            lines.push(format_compact_element(elem, 0));
        }
        lines.push(String::new());
    }

    lines.push(format!(
        "Stats: {} total, {} visible, {} actionable",
        summary.stats.total_elements,
        summary.stats.visible_elements,
        summary.stats.actionable_elements
    ));

    lines.join("\n")
}

/// Format a single compact element as text
fn format_compact_element(elem: &CompactElement, indent: usize) -> String {
    let prefix = "  ".repeat(indent);
    let mut parts = Vec::new();

    parts.push(format!("[{}]", elem.ref_id));
    parts.push(format!("{}:", elem.role));
    parts.push(format!("\"{}\"", elem.label));

    if let Some(ref action) = elem.action {
        parts.push(format!("({})", action));
    }

    if let Some(ref value) = elem.value {
        parts.push(format!("= \"{}\"", value));
    }

    if let Some(ref state) = elem.state {
        parts.push(format!("[{}]", state));
    }

    let mut line = format!("{}{}", prefix, parts.join(" "));

    // Add children
    for child in &elem.children {
        line.push('\n');
        line.push_str(&format_compact_element(child, indent + 1));
    }

    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_role_button() {
        let elem = UiaElement {
            control_type: "Button".to_string(),
            patterns: vec!["Invoke".to_string()],
            ..Default::default()
        };
        assert_eq!(infer_role(&elem), SemanticRole::Button);
    }

    #[test]
    fn test_infer_role_input() {
        let elem = UiaElement {
            control_type: "Edit".to_string(),
            patterns: vec!["Value".to_string()],
            ..Default::default()
        };
        assert_eq!(infer_role(&elem), SemanticRole::TextInput);
    }

    #[test]
    fn test_filter_noise() {
        let elem = UiaElement {
            name: String::new(),
            patterns: Vec::new(),
            children: Vec::new(),
            automation_id: String::new(),
            ..Default::default()
        };
        assert!(FilterHeuristics::is_noise(&elem));
    }

    #[test]
    fn test_ref_id_generator() {
        let mut gen = RefIdGenerator::new();
        assert_eq!(gen.next(SemanticRole::Button), "b1");
        assert_eq!(gen.next(SemanticRole::Button), "b2");
        assert_eq!(gen.next(SemanticRole::TextInput), "i1");
    }
}
