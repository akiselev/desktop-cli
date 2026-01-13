//! Direct operation implementations
//!
//! This module contains the actual implementation of desktop automation operations,
//! extracted from the RPC service for direct invocation without a daemon.

#[cfg(windows)]
mod windows_ops;

#[cfg(windows)]
pub use windows_ops::*;

#[cfg(not(windows))]
mod stub_ops;

#[cfg(not(windows))]
pub use stub_ops::*;
