//! Types for the automation agent

use serde::{Deserialize, Serialize};

/// An action the agent can take
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentAction {
    /// Click at coordinates or on described element
    Click {
        /// Target description or coordinates "x,y"
        target: String,
        /// Click type: left, right, double
        #[serde(default = "default_click_type")]
        click_type: String,
    },
    /// Type text
    Type {
        /// Text to type
        text: String,
        /// Optional target to focus first
        #[serde(default)]
        target: Option<String>,
    },
    /// Send key combination
    Keys {
        /// Key combo like "ctrl+c", "enter", "tab"
        keys: String,
    },
    /// Scroll in a direction
    Scroll {
        /// Direction: up, down
        direction: String,
        /// Number of scroll notches
        #[serde(default = "default_scroll_amount")]
        amount: i32,
    },
    /// Wait for a moment (milliseconds)
    Wait {
        #[serde(default = "default_wait_ms")]
        ms: u64,
    },
    /// Goal has been achieved
    Done {
        /// Explanation of what was accomplished
        reason: String,
    },
    /// Goal cannot be achieved
    Fail {
        /// Why the goal cannot be achieved
        reason: String,
    },
}

fn default_click_type() -> String {
    "left".to_string()
}

fn default_scroll_amount() -> i32 {
    3
}

fn default_wait_ms() -> u64 {
    500
}

/// Response from the LLM planner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerResponse {
    /// Brief reasoning about the current state and what to do
    pub reasoning: String,
    /// The action to take
    #[serde(flatten)]
    pub action: AgentAction,
}

/// A step in the agent execution history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    /// Step number (1-indexed)
    pub step: usize,
    /// What the agent decided to do
    pub action: AgentAction,
    /// Brief reasoning
    pub reasoning: String,
    /// Whether the action succeeded
    pub success: bool,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Final result of agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// Whether the goal was achieved
    pub success: bool,
    /// Summary of what happened
    pub summary: String,
    /// Number of steps taken
    pub steps_taken: usize,
    /// History of all steps
    pub history: Vec<AgentStep>,
    /// Final status: "done", "failed", "max_steps", "error"
    pub status: String,
}

impl AgentResult {
    pub fn success(summary: String, history: Vec<AgentStep>) -> Self {
        Self {
            success: true,
            summary,
            steps_taken: history.len(),
            history,
            status: "done".to_string(),
        }
    }

    pub fn failed(summary: String, history: Vec<AgentStep>, status: &str) -> Self {
        Self {
            success: false,
            summary,
            steps_taken: history.len(),
            history,
            status: status.to_string(),
        }
    }
}

/// Configuration for the agent
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Maximum number of steps before giving up
    pub max_steps: usize,
    /// Whether to include UI tree summary in context
    pub include_ui_summary: bool,
    /// Whether to include screenshot in context
    pub include_screenshot: bool,
    /// Delay between actions in milliseconds
    pub action_delay_ms: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 20,
            include_ui_summary: true,
            include_screenshot: true,
            action_delay_ms: 300,
        }
    }
}
