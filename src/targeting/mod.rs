//! Window targeting module
//!
//! Provides parsing, resolution, and suggestion functionality for window queries.
//! This allows users to target windows using intuitive syntax like:
//! - `:1`, `:2` - by index
//! - `notepad` - by executable name
//! - `title:PCB` - by title
//! - `hwnd:0x1234` - by exact HWND

mod parser;
mod resolver;
mod suggest;

pub use parser::WindowQuery;
pub use resolver::{resolve_window, resolve_with_element, ResolutionError};
pub use suggest::{
    format_suggestions, format_window_list, format_window_list_json,
};

