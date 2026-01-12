//! UI Automation module for Windows accessibility APIs
//!
//! Provides:
//! - Element tree traversal and dumping
//! - CSS-style selectors with wildcard support
//! - UIA pattern execution (Invoke, Value, Toggle, etc.)

mod element;
mod patterns;
mod selector;
mod tree;

pub use element::*;
pub use patterns::*;
pub use selector::*;
pub use tree::*;
