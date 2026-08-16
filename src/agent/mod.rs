//! LLM-driven automation agent.
//!
//! The semantic module is model-independent and is the canonical planner contract.
//! The Gemini planner remains as a compatibility adapter while it is migrated.

pub mod planner;
pub mod semantic;
pub mod types;

pub use planner::AgentPlanner;
pub use semantic::{AgentCommand as SemanticAgentCommand, PlannerContext as SemanticPlannerContext};
pub use types::*;
