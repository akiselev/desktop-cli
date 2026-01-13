//! LLM-driven automation agent
//!
//! Takes natural language instructions and uses Gemini to plan and execute
//! a series of UI actions to achieve the goal.

pub mod planner;
pub mod types;

pub use planner::AgentPlanner;
pub use types::*;
