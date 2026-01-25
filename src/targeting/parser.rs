//! Window query parsing
//!
//! Parses user-friendly window query syntax into structured WindowQuery.

use std::fmt;

/// Error when parsing a window query
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Unknown prefix type (e.g., "foo:bar" where "foo" isn't recognized)
    UnknownFilterType(String),
    /// Invalid index format
    InvalidIndex(String),
    /// Invalid HWND format
    InvalidHwnd(String),
    /// Invalid PID format
    InvalidPid(String),
    /// Empty query
    EmptyQuery,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnknownFilterType(t) => write!(f, "Unknown filter type: '{}'", t),
            ParseError::InvalidIndex(s) => write!(f, "Invalid index: '{}'", s),
            ParseError::InvalidHwnd(s) => write!(f, "Invalid HWND: '{}'", s),
            ParseError::InvalidPid(s) => write!(f, "Invalid PID: '{}'", s),
            ParseError::EmptyQuery => write!(f, "Empty window query"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Index specification for window selection
#[derive(Debug, Clone, PartialEq)]
pub enum IndexSpec {
    /// Numeric index (1-based)
    Number(usize),
    /// First window
    First,
    /// Last window
    Last,
}

/// Wildcard pattern for matching strings
#[derive(Debug, Clone)]
pub struct WildcardPattern {
    /// The raw pattern string
    pub pattern: String,
    /// Whether pattern starts with wildcard
    prefix_wild: bool,
    /// Whether pattern ends with wildcard
    suffix_wild: bool,
    /// Core text to match (without wildcards)
    core: String,
}

impl WildcardPattern {
    pub fn new(pattern: &str) -> Self {
        let prefix_wild = pattern.starts_with('*');
        let suffix_wild = pattern.ends_with('*');

        let core = pattern
            .trim_start_matches('*')
            .trim_end_matches('*')
            .to_lowercase();

        Self {
            pattern: pattern.to_string(),
            prefix_wild,
            suffix_wild,
            core,
        }
    }

    /// Check if a string matches this pattern (case-insensitive)
    pub fn matches(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();

        match (self.prefix_wild, self.suffix_wild) {
            // *foo* - contains
            (true, true) => text_lower.contains(&self.core),
            // *foo - ends with
            (true, false) => text_lower.ends_with(&self.core),
            // foo* - starts with
            (false, true) => text_lower.starts_with(&self.core),
            // foo - exact match (case-insensitive)
            (false, false) => text_lower == self.core,
        }
    }
}

/// Parsed window query with all filter criteria
#[derive(Debug, Clone, Default)]
pub struct WindowQuery {
    /// Match by index (:1, :2, :first, :last)
    pub index: Option<IndexSpec>,
    /// Match exe name (substring, case-insensitive)
    pub exe: Option<String>,
    /// Match title (with optional wildcards)
    pub title: Option<WildcardPattern>,
    /// Match exact HWND
    pub hwnd: Option<String>,
    /// Match PID
    pub pid: Option<u32>,
    /// Match window class
    pub class: Option<String>,
    /// General query (matches exe OR title)
    pub any: Option<String>,
}

impl WindowQuery {
    /// Parse a window query string
    ///
    /// # Examples
    /// - `:1` → index 1
    /// - `:first` → first window
    /// - `notepad` → exe or title contains "notepad"
    /// - `title:PCB` → title contains "PCB"
    /// - `exe:altium` → exe contains "altium"
    /// - `hwnd:0x1234` → exact HWND
    /// - `pid:12345` → process ID
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::EmptyQuery);
        }

        let mut query = WindowQuery::default();

        // Check for index syntax (:1, :first, :last)
        if let Some(index_str) = input.strip_prefix(':') {
            query.index = Some(parse_index(index_str)?);
            return Ok(query);
        }

        // Check for prefixed filters
        if let Some((prefix, value)) = input.split_once(':') {
            let prefix_lower = prefix.to_lowercase();
            match prefix_lower.as_str() {
                "title" | "t" => {
                    query.title = Some(WildcardPattern::new(value));
                }
                "exe" | "e" => {
                    query.exe = Some(value.to_lowercase());
                }
                "hwnd" | "h" => {
                    // Validate HWND format
                    if !is_valid_hwnd(value) {
                        return Err(ParseError::InvalidHwnd(value.to_string()));
                    }
                    query.hwnd = Some(value.to_string());
                }
                "pid" | "p" => {
                    let pid = value
                        .parse::<u32>()
                        .map_err(|_| ParseError::InvalidPid(value.to_string()))?;
                    query.pid = Some(pid);
                }
                "class" | "c" => {
                    query.class = Some(value.to_string());
                }
                _ => {
                    // Could be part of a title like "C:\path" - treat as general query
                    query.any = Some(input.to_lowercase());
                }
            }
        } else {
            // No prefix - general query matching exe or title
            query.any = Some(input.to_lowercase());
        }

        Ok(query)
    }

    /// Check if this query is empty (no filters set)
    pub fn is_empty(&self) -> bool {
        self.index.is_none()
            && self.exe.is_none()
            && self.title.is_none()
            && self.hwnd.is_none()
            && self.pid.is_none()
            && self.class.is_none()
            && self.any.is_none()
    }
}

impl fmt::Display for WindowQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if let Some(ref idx) = self.index {
            match idx {
                IndexSpec::Number(n) => parts.push(format!(":{}", n)),
                IndexSpec::First => parts.push(":first".to_string()),
                IndexSpec::Last => parts.push(":last".to_string()),
            }
        }
        if let Some(ref exe) = self.exe {
            parts.push(format!("exe:{}", exe));
        }
        if let Some(ref title) = self.title {
            parts.push(format!("title:{}", title.pattern));
        }
        if let Some(ref hwnd) = self.hwnd {
            parts.push(format!("hwnd:{}", hwnd));
        }
        if let Some(pid) = self.pid {
            parts.push(format!("pid:{}", pid));
        }
        if let Some(ref class) = self.class {
            parts.push(format!("class:{}", class));
        }
        if let Some(ref any) = self.any {
            parts.push(any.clone());
        }

        write!(f, "{}", parts.join(" "))
    }
}

fn parse_index(s: &str) -> Result<IndexSpec, ParseError> {
    match s.to_lowercase().as_str() {
        "first" | "1" => Ok(IndexSpec::Number(1)),
        "last" => Ok(IndexSpec::Last),
        _ => {
            let n = s
                .parse::<usize>()
                .map_err(|_| ParseError::InvalidIndex(s.to_string()))?;
            if n == 0 {
                return Err(ParseError::InvalidIndex("0 (index is 1-based)".to_string()));
            }
            Ok(IndexSpec::Number(n))
        }
    }
}

fn is_valid_hwnd(s: &str) -> bool {
    // Accept hex (0x1234) or decimal
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).is_ok()
    } else {
        s.parse::<u64>().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_index() {
        let q = WindowQuery::parse(":1").unwrap();
        assert_eq!(q.index, Some(IndexSpec::Number(1)));

        let q = WindowQuery::parse(":first").unwrap();
        assert_eq!(q.index, Some(IndexSpec::Number(1)));

        let q = WindowQuery::parse(":last").unwrap();
        assert_eq!(q.index, Some(IndexSpec::Last));

        let q = WindowQuery::parse(":42").unwrap();
        assert_eq!(q.index, Some(IndexSpec::Number(42)));
    }

    #[test]
    fn test_parse_exe() {
        let q = WindowQuery::parse("notepad").unwrap();
        assert_eq!(q.any, Some("notepad".to_string()));

        let q = WindowQuery::parse("exe:altium").unwrap();
        assert_eq!(q.exe, Some("altium".to_string()));
    }

    #[test]
    fn test_parse_title() {
        // Exact match (case-insensitive)
        let q = WindowQuery::parse("title:PCB").unwrap();
        assert!(q.title.is_some());
        assert!(q.title.as_ref().unwrap().matches("pcb"));
        assert!(q.title.as_ref().unwrap().matches("PCB"));
        assert!(!q.title.as_ref().unwrap().matches("My PCB Design")); // exact match, not contains

        // Contains match with wildcards
        let q = WindowQuery::parse("title:*PCB*").unwrap();
        assert!(q.title.as_ref().unwrap().matches("My PCB Design"));

        let q = WindowQuery::parse("title:*Draft*").unwrap();
        assert!(q.title.as_ref().unwrap().matches("Draft Document"));
        assert!(q.title.as_ref().unwrap().matches("My Draft"));
    }

    #[test]
    fn test_parse_hwnd() {
        let q = WindowQuery::parse("hwnd:0x1234").unwrap();
        assert_eq!(q.hwnd, Some("0x1234".to_string()));

        let q = WindowQuery::parse("hwnd:12345").unwrap();
        assert_eq!(q.hwnd, Some("12345".to_string()));
    }

    #[test]
    fn test_wildcard_pattern() {
        let p = WildcardPattern::new("*foo*");
        assert!(p.matches("contains foo here"));
        assert!(p.matches("FOO"));
        assert!(!p.matches("bar"));

        let p = WildcardPattern::new("foo*");
        assert!(p.matches("foobar"));
        assert!(!p.matches("barfoo"));

        let p = WildcardPattern::new("*foo");
        assert!(p.matches("barfoo"));
        assert!(!p.matches("foobar"));
    }
}
