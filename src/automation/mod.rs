//! Platform-specific desktop automation implementations
//!
//! This module provides cross-platform desktop automation using:
//! - Windows: UI Automation (UIA) API
//! - Linux: AT-SPI2 via D-Bus + X11
//! - macOS: Accessibility API (AXUIElement) + Core Graphics

pub mod types;

#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::*;
