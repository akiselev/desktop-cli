//! UI Automation module for Windows accessibility APIs
//!
//! Provides:
//! - Element tree traversal and dumping
//! - CSS-style selectors with wildcard support
//! - UIA pattern execution (Invoke, Value, Toggle, etc.)

pub mod element;
pub mod patterns;
pub mod selector;
pub mod tree;

pub use element::*;
pub use patterns::*;
pub use selector::*;
pub use tree::*;
