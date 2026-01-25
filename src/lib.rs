// Library exports for testing and external use
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod agent;
pub mod automation;
pub mod error;
pub mod executor;
pub mod gemini;
pub mod ops;
pub mod rpc;
pub mod targeting;

// Re-export commonly used types
pub use error::{DesktopCliError, Result};
