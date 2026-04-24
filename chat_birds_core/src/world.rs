use std::collections::HashMap;

use crate::agent::Agent;
use crate::core::AgentId;
use crate::message::{Message, MessageCodec};

/// The World trait manages agent simulation and message dispatch.
///
/// A world is responsible for:
/// - Storing and managing agents
/// - Providing a message codec for serialization
/// - Dispatching messages between agents (routing and delivery)
pub trait World {
    /// Get the message codec for this world (if any).
    fn codec(&self) -> Option<impl MessageCodec>;

    /// Get a reference to all agents.
    fn agents(&self) -> &HashMap<AgentId, Box<dyn Agent>>;

    /// Get a mutable reference to all agents.
    fn agents_mut(&mut self) -> &mut HashMap<AgentId, Box<dyn Agent>>;

    /// Dispatch a message to its recipient. Handles routing and chains responses.
    ///
    /// Uses a simple FIFO queue: processes the initial message's recipient,
    /// collects their responses, and queues them for delivery.
    fn dispatch(&mut self, initial: Message) {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(initial);
        while let Some(msg) = queue.pop_front() {
            let Some(recipient) = self.agents_mut().get_mut(&msg.to) else {
                continue;
            };
            let responses = recipient.on_message(msg);
            queue.extend(responses);
        }
    }

    /// Decode a string message and dispatch it.
    /// Returns true if decoding and dispatch succeeded.
    fn dispatch_from_str(&mut self, s: &str, from: AgentId, to: AgentId) -> bool {
        let Some(codec) = self.codec() else {
            return false;
        };
        let Some(msg) = codec.decode(s, from, to) else {
            return false;
        };
        drop(codec);
        self.dispatch(msg);
        true
    }
}
