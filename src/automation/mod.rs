pub mod types;

#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;
