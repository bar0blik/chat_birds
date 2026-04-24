use std::collections::HashMap;

use crate::core::AgentId;

/// Subjective reliability, used in SourceMap
#[derive(Clone, Copy)]
pub struct Trust(u8);

/// Holds trust levels of known source agents
pub struct SourceMap(HashMap<AgentId, Trust>);

impl SourceMap {
    /// Get a source agent's trust level.
    pub fn get(&self, source: AgentId) -> Option<Trust> {
        self.0.get(&source).copied()
    }

    /// Set a source agent's trust level and return the previous trust level.
    pub fn set(&mut self, source: AgentId, trust: Trust) -> Option<Trust> {
        self.0.insert(source, trust)
    }
}
