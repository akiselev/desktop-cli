use crate::semantic::{Observation, SemanticAction};
use serde::{Deserialize, Serialize};

/// Model-independent action schema consumed by external planners.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag="action", rename_all="kebab-case")]
pub enum AgentCommand {
    Perform { target: String, operation: SemanticAction, value: Option<String> },
    Query { selector: String },
    Observe,
    Wait { milliseconds: u64 },
    Done { reason: String },
    Fail { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerContext {
    pub goal: String,
    pub observation: Observation,
    pub history: Vec<AgentHistoryEntry>,
    pub available_pack_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHistoryEntry {
    pub command: AgentCommand,
    pub success: bool,
    pub summary: String,
}

impl PlannerContext {
    pub fn system_guidance(&self) -> String {
        "Use semantic element refs or $pack aliases from the observation. Prefer accessibility actions over coordinate input. Take one action at a time, observe the resulting revision, and never infer controls hidden by the active pack unless you explicitly query the raw semantic graph.".into()
    }
}
