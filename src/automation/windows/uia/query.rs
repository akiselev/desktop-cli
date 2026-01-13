//! Enhanced query language optimized for LLM ergonomics
//!
//! This module provides a more intuitive query syntax than raw CSS selectors,
//! designed to be easier for LLMs to generate and understand.
//!
//! # Query Syntax
//!
//! ## Role-based shortcuts (prefix: @)
//! - `@button` - All buttons
//! - `@input` - All text inputs
//! - `@checkbox` - All checkboxes
//! - `@menu` - Menu items
//! - `@tab` - Tab items
//! - `@link` - Hyperlinks
//! - `@list` - List items
//! - `@tree` - Tree items
//!
//! ## Text matching (prefix: ")
//! - `"Save"` - Element with exact name "Save"
//! - `"*Save*"` - Element containing "Save"
//! - `"Save..."` - Element starting with "Save"
//!
//! ## ID/automation ID (prefix: #)
//! - `#btnSave` - Element with automation ID "btnSave"
//!
//! ## Pseudo-selectors (prefix: :)
//! - `:focus` - Currently focused element
//! - `:enabled` - Only enabled elements
//! - `:disabled` - Only disabled elements
//! - `:nth(N)` - Nth match (1-based)
//! - `:first` - First match (alias for :nth(1))
//! - `:last` - Last match
//! - `:contains(text)` - Element containing text
//! - `:value(text)` - Element with specific value
//!
//! ## Spatial selectors (prefix: ~)
//! - `~near(selector)` - Elements near the matched element
//! - `~below(selector)` - Elements below the matched element
//! - `~right(selector)` - Elements to the right of matched element
//! - `~inside(selector)` - Elements inside (children of) matched element
//!
//! ## Combinators
//! - `@button "Save"` - Button with name containing "Save"
//! - `@input:enabled` - Enabled input fields
//! - `#toolbar @button` - Buttons inside element with ID "toolbar"
//!
//! ## Examples
//! ```text
//! @button "Save"           -> Click the Save button
//! @input "Username"        -> Username input field
//! @menu "File" > "Open"    -> File > Open menu path
//! @tab:nth(2)              -> Second tab
//! ~below("Username") @input -> Input field below "Username" label
//! #mainPanel @button:first  -> First button in mainPanel
//! ```

use super::selector::{AttributeMatcher, MatchOp, Selector, SelectorSegment};
use crate::rpc::types::UiaElement;

/// Parsed query with all components
#[derive(Debug, Clone)]
pub struct Query {
    /// The core selector(s)
    pub selector: Selector,
    /// Index filter (from :nth)
    pub index: Option<QueryIndex>,
    /// State filters
    pub state_filters: Vec<StateFilter>,
    /// Spatial relation
    pub spatial: Option<SpatialQuery>,
}

/// Index-based selection
#[derive(Debug, Clone)]
pub enum QueryIndex {
    /// Specific index (1-based)
    Nth(usize),
    /// First match
    First,
    /// Last match
    Last,
}

/// State filter
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StateFilter {
    Enabled,
    Disabled,
    Focused,
    Visible,
    Hidden,
}

/// Spatial relationship query
#[derive(Debug, Clone)]
pub struct SpatialQuery {
    pub relation: SpatialRelation,
    pub anchor: String, // Selector for anchor element
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpatialRelation {
    Near,
    Below,
    Above,
    Left,
    Right,
    Inside,
}

/// Role shortcut mappings
pub fn role_to_control_types(role: &str) -> Vec<&'static str> {
    match role.to_lowercase().as_str() {
        "button" | "btn" => vec!["Button"],
        "input" | "text" | "textbox" | "edit" => vec!["Edit", "Document"],
        "checkbox" | "check" => vec!["CheckBox"],
        "radio" => vec!["RadioButton"],
        "dropdown" | "combo" | "select" => vec!["ComboBox"],
        "menu" | "menuitem" => vec!["MenuItem", "Menu"],
        "tab" => vec!["TabItem"],
        "link" | "hyperlink" => vec!["Hyperlink"],
        "list" | "listitem" => vec!["ListItem", "DataItem"],
        "tree" | "treeitem" => vec!["TreeItem"],
        "slider" | "range" => vec!["Slider", "Spinner"],
        "table" | "grid" => vec!["DataGrid", "Table"],
        "toolbar" => vec!["ToolBar"],
        "dialog" | "modal" => vec!["Window", "Pane"],
        _ => vec![],
    }
}

/// Parse an enhanced query string into a Query
pub fn parse_query(input: &str) -> Result<Query, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("Empty query".to_string());
    }

    let mut selector_parts = Vec::new();
    let mut index = None;
    let mut state_filters = Vec::new();
    let mut spatial = None;

    // Tokenize and parse
    let tokens = tokenize(input);
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        match token.as_str() {
            // Role-based: @button, @input, etc.
            t if t.starts_with('@') => {
                let role = &t[1..];
                let control_types = role_to_control_types(role);
                if control_types.is_empty() {
                    return Err(format!("Unknown role: @{}", role));
                }
                // Use first control type for now
                selector_parts.push(SelectorSegment {
                    control_type: Some(control_types[0].to_string()),
                    ..Default::default()
                });
            }

            // Text match: "Save", "*Save*", etc.
            t if t.starts_with('"') && t.ends_with('"') => {
                let text = &t[1..t.len() - 1];
                let matcher = if text.starts_with('*') && text.ends_with('*') {
                    AttributeMatcher {
                        name: "name".to_string(),
                        op: MatchOp::Contains,
                        value: text[1..text.len() - 1].to_string(),
                    }
                } else if text.ends_with("...") {
                    AttributeMatcher {
                        name: "name".to_string(),
                        op: MatchOp::StartsWith,
                        value: text[..text.len() - 3].to_string(),
                    }
                } else {
                    AttributeMatcher {
                        name: "name".to_string(),
                        op: MatchOp::Exact,
                        value: text.to_string(),
                    }
                };

                if let Some(last) = selector_parts.last_mut() {
                    last.attributes.push(matcher);
                } else {
                    selector_parts.push(SelectorSegment {
                        attributes: vec![matcher],
                        ..Default::default()
                    });
                }
            }

            // Automation ID: #btnSave
            t if t.starts_with('#') => {
                let id = &t[1..];
                if let Some(last) = selector_parts.last_mut() {
                    last.automation_id = Some(id.to_string());
                } else {
                    selector_parts.push(SelectorSegment {
                        automation_id: Some(id.to_string()),
                        ..Default::default()
                    });
                }
            }

            // Pseudo-selectors: :nth(N), :enabled, :focus, etc.
            t if t.starts_with(':') => {
                let pseudo = &t[1..];
                if pseudo == "enabled" {
                    state_filters.push(StateFilter::Enabled);
                } else if pseudo == "disabled" {
                    state_filters.push(StateFilter::Disabled);
                } else if pseudo == "focus" || pseudo == "focused" {
                    state_filters.push(StateFilter::Focused);
                } else if pseudo == "visible" {
                    state_filters.push(StateFilter::Visible);
                } else if pseudo == "hidden" {
                    state_filters.push(StateFilter::Hidden);
                } else if pseudo == "first" {
                    index = Some(QueryIndex::First);
                } else if pseudo == "last" {
                    index = Some(QueryIndex::Last);
                } else if pseudo.starts_with("nth(") && pseudo.ends_with(')') {
                    let n_str = &pseudo[4..pseudo.len() - 1];
                    if let Ok(n) = n_str.parse::<usize>() {
                        index = Some(QueryIndex::Nth(n));
                    }
                } else if pseudo.starts_with("contains(") && pseudo.ends_with(')') {
                    let text = &pseudo[9..pseudo.len() - 1];
                    let text = text.trim_matches('"').trim_matches('\'');
                    let matcher = AttributeMatcher {
                        name: "name".to_string(),
                        op: MatchOp::Contains,
                        value: text.to_string(),
                    };
                    if let Some(last) = selector_parts.last_mut() {
                        last.attributes.push(matcher);
                    }
                } else if pseudo.starts_with("value(") && pseudo.ends_with(')') {
                    let text = &pseudo[6..pseudo.len() - 1];
                    let text = text.trim_matches('"').trim_matches('\'');
                    let matcher = AttributeMatcher {
                        name: "value".to_string(),
                        op: MatchOp::Exact,
                        value: text.to_string(),
                    };
                    if let Some(last) = selector_parts.last_mut() {
                        last.attributes.push(matcher);
                    }
                }
            }

            // Spatial queries: ~below("label"), ~near(#id)
            t if t.starts_with('~') => {
                let rest = &t[1..];
                if let Some(paren_idx) = rest.find('(') {
                    let relation_str = &rest[..paren_idx];
                    let anchor = &rest[paren_idx + 1..rest.len() - 1];
                    let relation = match relation_str {
                        "near" => SpatialRelation::Near,
                        "below" => SpatialRelation::Below,
                        "above" => SpatialRelation::Above,
                        "left" => SpatialRelation::Left,
                        "right" => SpatialRelation::Right,
                        "inside" => SpatialRelation::Inside,
                        _ => return Err(format!("Unknown spatial relation: {}", relation_str)),
                    };
                    spatial = Some(SpatialQuery {
                        relation,
                        anchor: anchor.to_string(),
                    });
                }
            }

            // Direct child combinator
            ">" => {
                if let Some(last) = selector_parts.last_mut() {
                    last.is_direct_child = true;
                }
            }

            // Control type (legacy CSS-style)
            t if t.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) => {
                selector_parts.push(SelectorSegment {
                    control_type: Some(t.to_string()),
                    ..Default::default()
                });
            }

            _ => {}
        }

        i += 1;
    }

    if selector_parts.is_empty() {
        return Err("No valid selector found in query".to_string());
    }

    Ok(Query {
        selector: Selector {
            segments: selector_parts,
        },
        index,
        state_filters,
        spatial,
    })
}

/// Tokenize query string, handling quoted strings
fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut in_parens = 0;

    for c in input.chars() {
        match c {
            '"' if in_parens == 0 => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            '(' => {
                in_parens += 1;
                current.push(c);
            }
            ')' => {
                in_parens -= 1;
                current.push(c);
            }
            ' ' if !in_quotes && in_parens == 0 => {
                if !current.is_empty() {
                    tokens.push(current);
                    current = String::new();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Apply state filters to elements
pub fn apply_state_filters(elements: &[UiaElement], filters: &[StateFilter]) -> Vec<UiaElement> {
    elements
        .iter()
        .filter(|elem| {
            for filter in filters {
                match filter {
                    StateFilter::Enabled => {
                        if !elem.is_enabled {
                            return false;
                        }
                    }
                    StateFilter::Disabled => {
                        if elem.is_enabled {
                            return false;
                        }
                    }
                    StateFilter::Visible => {
                        if elem.is_offscreen {
                            return false;
                        }
                    }
                    StateFilter::Hidden => {
                        if !elem.is_offscreen {
                            return false;
                        }
                    }
                    StateFilter::Focused => {
                        // Would need actual focus state from UIA
                    }
                }
            }
            true
        })
        .cloned()
        .collect()
}

/// Apply index filter to elements
pub fn apply_index_filter(elements: Vec<UiaElement>, index: &QueryIndex) -> Vec<UiaElement> {
    match index {
        QueryIndex::First => elements.into_iter().take(1).collect(),
        QueryIndex::Last => elements.into_iter().last().into_iter().collect(),
        QueryIndex::Nth(n) => {
            if *n > 0 && *n <= elements.len() {
                vec![elements[n - 1].clone()]
            } else {
                vec![]
            }
        }
    }
}

/// Check if element B is spatially related to anchor A
pub fn check_spatial_relation(anchor: &UiaElement, candidate: &UiaElement, relation: SpatialRelation) -> bool {
    let [ax, ay, aw, ah] = anchor.bounds;
    let [bx, by, bw, bh] = candidate.bounds;

    let a_center_x = ax + aw / 2;
    let a_center_y = ay + ah / 2;
    let b_center_x = bx + bw / 2;
    let b_center_y = by + bh / 2;

    match relation {
        SpatialRelation::Below => {
            // B is below A if B's top is below A's bottom
            by > ay + ah && (bx < ax + aw && bx + bw > ax) // Overlapping horizontally
        }
        SpatialRelation::Above => {
            by + bh < ay && (bx < ax + aw && bx + bw > ax)
        }
        SpatialRelation::Right => {
            bx > ax + aw && (by < ay + ah && by + bh > ay)
        }
        SpatialRelation::Left => {
            bx + bw < ax && (by < ay + ah && by + bh > ay)
        }
        SpatialRelation::Near => {
            // Within 100 pixels
            let dist_x = (a_center_x - b_center_x).abs();
            let dist_y = (a_center_y - b_center_y).abs();
            dist_x < 100 && dist_y < 100
        }
        SpatialRelation::Inside => {
            // B is inside A
            bx >= ax && by >= ay && bx + bw <= ax + aw && by + bh <= ay + ah
        }
    }
}

/// Generate a unique selector for an element
pub fn generate_selector(elem: &UiaElement) -> String {
    let mut parts = Vec::new();

    // Prefer automation ID if available
    if !elem.automation_id.is_empty() {
        return format!("#{}", elem.automation_id);
    }

    // Use control type
    if !elem.control_type.is_empty() {
        parts.push(elem.control_type.clone());
    }

    // Add name if available
    if !elem.name.is_empty() {
        // Escape quotes and special chars
        let escaped = elem.name.replace('"', "\\\"");
        parts.push(format!("[name=\"{}\"]", escaped));
    }

    if parts.is_empty() {
        // Fall back to class name
        if !elem.class_name.is_empty() {
            return format!(".{}", elem.class_name);
        }
        return "*".to_string();
    }

    parts.join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_role() {
        let query = parse_query("@button").unwrap();
        assert_eq!(
            query.selector.segments[0].control_type,
            Some("Button".to_string())
        );
    }

    #[test]
    fn test_parse_role_with_text() {
        let query = parse_query("@button \"Save\"").unwrap();
        assert_eq!(query.selector.segments[0].attributes.len(), 1);
        assert_eq!(query.selector.segments[0].attributes[0].value, "Save");
    }

    #[test]
    fn test_parse_automation_id() {
        let query = parse_query("#btnSave").unwrap();
        assert_eq!(
            query.selector.segments[0].automation_id,
            Some("btnSave".to_string())
        );
    }

    #[test]
    fn test_parse_nth() {
        let query = parse_query("@button:nth(3)").unwrap();
        assert!(matches!(query.index, Some(QueryIndex::Nth(3))));
    }

    #[test]
    fn test_parse_state_filter() {
        let query = parse_query("@input:enabled").unwrap();
        assert!(query.state_filters.contains(&StateFilter::Enabled));
    }

    #[test]
    fn test_parse_contains() {
        let query = parse_query("\"*Save*\"").unwrap();
        assert_eq!(query.selector.segments[0].attributes[0].op, MatchOp::Contains);
    }

    #[test]
    fn test_tokenize_quoted() {
        let tokens = tokenize("@button \"Save Changes\"");
        assert_eq!(tokens, vec!["@button", "\"Save Changes\""]);
    }
}
