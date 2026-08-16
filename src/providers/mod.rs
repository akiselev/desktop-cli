pub mod legacy;
pub mod linux;
pub mod macos;
pub mod mock;
pub mod windows;
#[cfg(windows)]
pub mod windows_uia;

pub use legacy::*;
pub use mock::*;
#[cfg(windows)]
pub use windows_uia::*;
