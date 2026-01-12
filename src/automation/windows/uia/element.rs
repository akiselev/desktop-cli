//! UIA element types - re-exports from RPC types for use in UIA module
//!
//! The actual type definitions are in `crate::rpc::types` so they're available
//! cross-platform for RPC serialization.

pub use crate::rpc::types::{PatternResult, TreeDumpOptions, UiaElement};
