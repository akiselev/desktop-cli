// Library exports for testing and external use

pub mod automation;
pub mod error;
pub mod ops;
pub mod rpc;
pub mod targeting;

// Re-export commonly used types
pub use error::{DesktopCliError, Result};

