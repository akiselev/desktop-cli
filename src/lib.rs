// Library exports for testing and external use

pub mod agent;
pub mod automation;
pub mod error;
pub mod executor;
pub mod gemini;
pub mod rpc;

// Re-export commonly used types
pub use error::{DesktopCliError, Result};
