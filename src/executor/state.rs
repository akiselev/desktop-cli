use serde::{Deserialize, Serialize};

/// Execution state for tracking multi-step instruction progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub current_step: usize,
    pub total_steps: usize,
    pub results: Vec<StepResult>,
}

impl ExecutionState {
    pub fn new(total_steps: usize) -> Self {
        Self {
            current_step: 0,
            total_steps,
            results: Vec::with_capacity(total_steps),
        }
    }

    /// Record a successful step
    pub fn record_success(&mut self, instruction: String, coordinates: Option<(i32, i32)>) {
        self.results.push(StepResult {
            instruction,
            status: StepStatus::Success,
            coordinates,
            error: None,
        });
        self.current_step += 1;
    }

    /// Record a failed step
    pub fn record_failure(&mut self, instruction: String, error: String) {
        self.results.push(StepResult {
            instruction,
            status: StepStatus::Failed,
            coordinates: None,
            error: Some(error),
        });
        self.current_step += 1;
    }

    /// Record a skipped step (e.g., due to earlier failure)
    pub fn record_skipped(&mut self, instruction: String, reason: String) {
        self.results.push(StepResult {
            instruction,
            status: StepStatus::Skipped,
            coordinates: None,
            error: Some(reason),
        });
        self.current_step += 1;
    }

    /// Check if execution is complete
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.total_steps
    }

    /// Count successful steps
    pub fn successful_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.status, StepStatus::Success))
            .count()
    }

    /// Count failed steps
    pub fn failed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.status, StepStatus::Failed))
            .count()
    }

    /// Get overall success status
    pub fn overall_success(&self) -> bool {
        self.failed_count() == 0 && self.successful_count() > 0
    }

    /// Convert to execution summary
    pub fn to_summary(&self) -> ExecutionSummary {
        ExecutionSummary {
            success: self.overall_success(),
            executed_steps: self.current_step,
            failed_steps: self.failed_count(),
            results: self.results.clone(),
        }
    }
}

/// Result of a single step in the execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub instruction: String,
    pub status: StepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordinates: Option<(i32, i32)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Success,
    Failed,
    Skipped,
}

/// Summary of execution to return to MCP client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub success: bool,
    pub executed_steps: usize,
    pub failed_steps: usize,
    pub results: Vec<StepResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_state_new() {
        let state = ExecutionState::new(5);
        assert_eq!(state.total_steps, 5);
        assert_eq!(state.current_step, 0);
        assert_eq!(state.results.len(), 0);
    }

    #[test]
    fn test_record_success() {
        let mut state = ExecutionState::new(3);
        state.record_success("click button".to_string(), Some((100, 200)));
        assert_eq!(state.current_step, 1);
        assert_eq!(state.successful_count(), 1);
        assert!(matches!(state.results[0].status, StepStatus::Success));
    }

    #[test]
    fn test_record_failure() {
        let mut state = ExecutionState::new(3);
        state.record_failure("click button".to_string(), "Element not found".to_string());
        assert_eq!(state.current_step, 1);
        assert_eq!(state.failed_count(), 1);
        assert!(matches!(state.results[0].status, StepStatus::Failed));
    }

    #[test]
    fn test_overall_success() {
        let mut state = ExecutionState::new(3);
        state.record_success("step 1".to_string(), None);
        state.record_success("step 2".to_string(), None);
        state.record_success("step 3".to_string(), None);
        assert!(state.overall_success());

        let mut state2 = ExecutionState::new(3);
        state2.record_success("step 1".to_string(), None);
        state2.record_failure("step 2".to_string(), "Error".to_string());
        assert!(!state2.overall_success());
    }

    #[test]
    fn test_execution_summary() {
        let mut state = ExecutionState::new(3);
        state.record_success("step 1".to_string(), None);
        state.record_failure("step 2".to_string(), "Error".to_string());

        let summary = state.to_summary();
        assert!(!summary.success);
        assert_eq!(summary.executed_steps, 2);
        assert_eq!(summary.failed_steps, 1);
        assert_eq!(summary.results.len(), 2);
    }
}
