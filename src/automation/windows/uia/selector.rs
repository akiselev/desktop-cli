//! CSS-style selector parsing and matching with wildcard support
//!
//! Selector syntax:
//! - `Type` - Control type (Button, Edit, List)
//! - `#id` - Automation ID
//! - `.class` - Class name
//! - `[attr=value]` - Exact attribute match
//! - `[attr*=value]` - Contains
//! - `[attr^=value]` - Starts with
//! - `[attr$=value]` - Ends with
//! - `[attr~=pattern]` - Wildcard match (*, ?)
//! - `A B` - Descendant
//! - `A > B` - Direct child
//! - `*` - Any type

use wildmatch::WildMatch;

/// Parsed selector segment (one part of a selector chain)
#[derive(Debug, Clone)]
pub struct SelectorSegment {
    /// Control type to match (e.g., "Button", "*" for any)
    pub control_type: Option<String>,
    /// Automation ID to match (from #id syntax)
    pub automation_id: Option<String>,
    /// Class name to match (from .class syntax)
    pub class_name: Option<String>,
    /// Attribute matchers
    pub attributes: Vec<AttributeMatcher>,
    /// Whether this is a direct child (>) or descendant (space)
    pub is_direct_child: bool,
}

/// Attribute matching specification
#[derive(Debug, Clone)]
pub struct AttributeMatcher {
    pub name: String,
    pub op: MatchOp,
    pub value: String,
}

/// Match operation for attributes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MatchOp {
    /// Exact match `=`
    Exact,
    /// Contains `*=`
    Contains,
    /// Starts with `^=`
    StartsWith,
    /// Ends with `$=`
    EndsWith,
    /// Wildcard match `~=`
    Wildcard,
}

/// Parsed CSS-style selector
#[derive(Debug, Clone)]
pub struct Selector {
    pub segments: Vec<SelectorSegment>,
}

impl Selector {
    /// Parse a CSS-style selector string
    pub fn parse(input: &str) -> Result<Self, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("Empty selector".to_string());
        }

        let mut segments = Vec::new();
        let mut current = SelectorSegment::default();
        let mut chars = input.chars().peekable();
        let mut in_bracket = false;
        let mut bracket_content = String::new();
        let mut is_direct_child = false;

        while let Some(c) = chars.next() {
            match c {
                // Direct child combinator
                '>' if !in_bracket => {
                    if current.has_content() {
                        current.is_direct_child = is_direct_child;
                        segments.push(current);
                        current = SelectorSegment::default();
                    }
                    is_direct_child = true;
                    // Skip whitespace after >
                    while chars.peek() == Some(&' ') {
                        chars.next();
                    }
                }
                // Descendant combinator (space)
                ' ' if !in_bracket => {
                    if current.has_content() {
                        current.is_direct_child = is_direct_child;
                        segments.push(current);
                        current = SelectorSegment::default();
                        is_direct_child = false;
                    }
                    // Skip additional whitespace
                    while chars.peek() == Some(&' ') {
                        chars.next();
                    }
                    // Check if next is >
                    if chars.peek() == Some(&'>') {
                        chars.next();
                        is_direct_child = true;
                        while chars.peek() == Some(&' ') {
                            chars.next();
                        }
                    }
                }
                // Automation ID
                '#' if !in_bracket => {
                    let id: String = chars
                        .by_ref()
                        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                        .collect();
                    if !id.is_empty() {
                        current.automation_id = Some(id);
                    }
                }
                // Class name
                '.' if !in_bracket => {
                    let class: String = chars
                        .by_ref()
                        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                        .collect();
                    if !class.is_empty() {
                        current.class_name = Some(class);
                    }
                }
                // Attribute selector start
                '[' => {
                    in_bracket = true;
                    bracket_content.clear();
                }
                // Attribute selector end
                ']' if in_bracket => {
                    in_bracket = false;
                    if let Some(matcher) = parse_attribute(&bracket_content) {
                        current.attributes.push(matcher);
                    }
                }
                // Universal selector
                '*' if !in_bracket && current.control_type.is_none() => {
                    current.control_type = Some("*".to_string());
                }
                // Content inside brackets
                _ if in_bracket => {
                    bracket_content.push(c);
                }
                // Control type (alphanumeric)
                _ if c.is_alphanumeric() && current.control_type.is_none() => {
                    let mut type_name = String::from(c);
                    while let Some(&next) = chars.peek() {
                        if next.is_alphanumeric() {
                            type_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    current.control_type = Some(type_name);
                }
                _ => {}
            }
        }

        // Don't forget the last segment
        if current.has_content() {
            current.is_direct_child = is_direct_child;
            segments.push(current);
        }

        if segments.is_empty() {
            return Err("No valid selector segments found".to_string());
        }

        Ok(Selector { segments })
    }
}

impl SelectorSegment {
    fn has_content(&self) -> bool {
        self.control_type.is_some()
            || self.automation_id.is_some()
            || self.class_name.is_some()
            || !self.attributes.is_empty()
    }
}

impl Default for SelectorSegment {
    fn default() -> Self {
        Self {
            control_type: None,
            automation_id: None,
            class_name: None,
            attributes: Vec::new(),
            is_direct_child: false,
        }
    }
}

/// Parse an attribute expression like `name="value"` or `name*="value"`
fn parse_attribute(content: &str) -> Option<AttributeMatcher> {
    let content = content.trim();

    // Find the operator
    let (name, op, value) = if let Some(idx) = content.find("~=") {
        (&content[..idx], MatchOp::Wildcard, &content[idx + 2..])
    } else if let Some(idx) = content.find("*=") {
        (&content[..idx], MatchOp::Contains, &content[idx + 2..])
    } else if let Some(idx) = content.find("^=") {
        (&content[..idx], MatchOp::StartsWith, &content[idx + 2..])
    } else if let Some(idx) = content.find("$=") {
        (&content[..idx], MatchOp::EndsWith, &content[idx + 2..])
    } else if let Some(idx) = content.find('=') {
        (&content[..idx], MatchOp::Exact, &content[idx + 1..])
    } else {
        return None;
    };

    let name = name.trim().to_string();
    let value = value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();

    if name.is_empty() {
        return None;
    }

    Some(AttributeMatcher { name, op, value })
}

impl AttributeMatcher {
    /// Check if a value matches this attribute matcher
    pub fn matches(&self, actual: &str) -> bool {
        match self.op {
            MatchOp::Exact => actual == self.value,
            MatchOp::Contains => actual.contains(&self.value),
            MatchOp::StartsWith => actual.starts_with(&self.value),
            MatchOp::EndsWith => actual.ends_with(&self.value),
            MatchOp::Wildcard => WildMatch::new(&self.value).matches(actual),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_type() {
        let sel = Selector::parse("Button").unwrap();
        assert_eq!(sel.segments.len(), 1);
        assert_eq!(sel.segments[0].control_type, Some("Button".to_string()));
    }

    #[test]
    fn test_parse_id() {
        let sel = Selector::parse("#saveButton").unwrap();
        assert_eq!(sel.segments.len(), 1);
        assert_eq!(
            sel.segments[0].automation_id,
            Some("saveButton".to_string())
        );
    }

    #[test]
    fn test_parse_class() {
        let sel = Selector::parse(".TextBox").unwrap();
        assert_eq!(sel.segments.len(), 1);
        assert_eq!(sel.segments[0].class_name, Some("TextBox".to_string()));
    }

    #[test]
    fn test_parse_combined() {
        let sel = Selector::parse("Button#save.Primary").unwrap();
        assert_eq!(sel.segments.len(), 1);
        assert_eq!(sel.segments[0].control_type, Some("Button".to_string()));
        assert_eq!(sel.segments[0].automation_id, Some("save".to_string()));
        assert_eq!(sel.segments[0].class_name, Some("Primary".to_string()));
    }

    #[test]
    fn test_parse_attribute() {
        let sel = Selector::parse("[name=\"OK\"]").unwrap();
        assert_eq!(sel.segments.len(), 1);
        assert_eq!(sel.segments[0].attributes.len(), 1);
        assert_eq!(sel.segments[0].attributes[0].name, "name");
        assert_eq!(sel.segments[0].attributes[0].op, MatchOp::Exact);
        assert_eq!(sel.segments[0].attributes[0].value, "OK");
    }

    #[test]
    fn test_parse_wildcard_attribute() {
        let sel = Selector::parse("[name~=\"*Save*\"]").unwrap();
        assert_eq!(sel.segments[0].attributes[0].op, MatchOp::Wildcard);
        assert!(sel.segments[0].attributes[0].matches("Click to Save"));
        assert!(sel.segments[0].attributes[0].matches("Save"));
        assert!(!sel.segments[0].attributes[0].matches("Cancel"));
    }

    #[test]
    fn test_parse_descendant() {
        let sel = Selector::parse("Window Button").unwrap();
        assert_eq!(sel.segments.len(), 2);
        assert_eq!(sel.segments[0].control_type, Some("Window".to_string()));
        assert_eq!(sel.segments[1].control_type, Some("Button".to_string()));
        assert!(!sel.segments[1].is_direct_child);
    }

    #[test]
    fn test_parse_direct_child() {
        let sel = Selector::parse("Pane > Button").unwrap();
        assert_eq!(sel.segments.len(), 2);
        assert_eq!(sel.segments[0].control_type, Some("Pane".to_string()));
        assert_eq!(sel.segments[1].control_type, Some("Button".to_string()));
        assert!(sel.segments[1].is_direct_child);
    }

    #[test]
    fn test_parse_universal() {
        let sel = Selector::parse("* Button").unwrap();
        assert_eq!(sel.segments[0].control_type, Some("*".to_string()));
    }

    #[test]
    fn test_wildcard_matching() {
        let matcher = AttributeMatcher {
            name: "name".to_string(),
            op: MatchOp::Wildcard,
            value: "???-????".to_string(),
        };
        assert!(matcher.matches("ABC-1234"));
        assert!(!matcher.matches("AB-1234"));
        assert!(!matcher.matches("ABCD-1234"));
    }
}
