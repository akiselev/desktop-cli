//! Linux desktop automation module
//!
//! Provides cross-platform desktop automation for Linux using:
//! - X11 for window enumeration and management
//! - AT-SPI2 (Assistive Technology Service Provider Interface) for accessibility tree
//! - XTest extension for input simulation
//! - X11 for screenshot capture

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
