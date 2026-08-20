use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationTelemetry {
    pub native_calls: u64,
    pub elements_inspected: u64,
    pub properties_fetched: u64,
    pub bytes_fetched: u64,
    pub text_bytes_fetched: u64,
    pub children_loaded: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub native_query_candidates: u64,
    pub residual_candidates: u64,
    pub graph_nodes_created: u64,
    pub graph_nodes_updated: u64,
    pub projected_nodes: u64,
    pub backend_micros: u64,
    pub projection_micros: u64,
    pub total_micros: u64,
}

pub struct OperationTimer { start: Instant }
impl OperationTimer {
    pub fn start() -> Self { Self { start: Instant::now() } }
    pub fn elapsed(&self) -> Duration { self.start.elapsed() }
    pub fn finish_into(self, target: &mut u64) { *target = self.start.elapsed().as_micros().min(u64::MAX as u128) as u64; }
}
