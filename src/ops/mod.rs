//! Direct operation implementations
//!
//! This module contains the actual implementation of desktop automation operations,
//! providing a unified API across Windows, Linux, and macOS.
//!
//! Platform-specific implementations:
//! - Windows: Uses UI Automation (UIA) API
//! - Linux: Uses AT-SPI2 via D-Bus + X11/XTest
//! - macOS: Uses Accessibility API (AXUIElement) + Core Graphics

#[cfg(windows)]
mod windows_ops;

#[cfg(windows)]
pub use windows_ops::*;

#[cfg(target_os = "linux")]
mod linux_ops;

#[cfg(target_os = "linux")]
pub use linux_ops::*;

#[cfg(target_os = "macos")]
mod macos_ops;

#[cfg(target_os = "macos")]
pub use macos_ops::*;

// Fallback stub for unsupported platforms (e.g., FreeBSD, etc.)
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
mod stub_ops;

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub use stub_ops::*;
