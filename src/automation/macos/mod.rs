//! macOS desktop automation module
//!
//! Provides cross-platform desktop automation for macOS using:
//! - Core Graphics for window enumeration and screenshots
//! - Accessibility API (AXUIElement) for accessibility tree
//! - Core Graphics Events for input simulation

pub mod accessibility;
pub mod coordinates;
pub mod input;
pub mod screenshot;
pub mod window;

pub use accessibility::*;
pub use coordinates::*;
pub use input::*;
pub use screenshot::*;
pub use window::*;
