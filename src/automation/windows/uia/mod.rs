//! UI Automation module for Windows accessibility APIs
//!
//! Provides:
//! - Element tree traversal and dumping
//! - CSS-style selectors with wildcard support
//! - Enhanced query language for LLM ergonomics
//! - UIA pattern execution (Invoke, Value, Toggle, etc.)
//! - Compact summary views optimized for LLM consumption

pub mod element;
pub mod patterns;
pub mod query;
pub mod selector;
pub mod summary;
pub mod tree;

pub use element::*;
pub use patterns::*;
pub use query::*;
pub use selector::*;
pub use summary::*;
pub use tree::*;
