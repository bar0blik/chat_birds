use crate::belief::BeliefStore;
use crate::core::AgentId;

/// A message passed between agents, carrying a payload of beliefs.
pub struct Message {
    pub from: AgentId,
    pub to: AgentId,
    pub payload: BeliefStore,
}

/// Trait for encoding and decoding messages to/from strings.
pub trait MessageCodec {
    fn encode(&self, msg: &Message) -> String;
    fn decode(&self, s: &str, from: AgentId, to: AgentId) -> Option<Message>;
}

/// Convert a belief state into human-readable text.
pub trait IntoUtterance {
    fn to_utterance(&self) -> String;
}

/// Parse human-readable text into a belief state.
pub trait FromUtterance: Sized {
    fn from_utterance(s: &str) -> Option<Self>;
}
