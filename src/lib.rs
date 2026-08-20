// Library exports for testing, embedding, and agent integrations.
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod agent;
pub mod automation;
pub mod error;
pub mod executor;
pub mod gemini;
pub mod ops;
pub mod packs;
pub mod providers;
pub mod rpc;
pub mod semantic;
pub mod session;
pub mod targeting;

pub use error::{DesktopCliError, Result};
