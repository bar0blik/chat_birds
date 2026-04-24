use std::collections::HashMap;

use crate::belief::{BeliefEntry, BeliefMap, BeliefSource, BeliefStore};
use crate::core::{AgentId, StateMap};
use crate::message::Message;

/// The Agent trait defines behavior for NPCs and other autonomous entities.
///
/// An agent has:
/// - Own states (immediate facts about itself)
/// - Beliefs (probabilistic knowledge about the world and other agents)
/// - Reactions to messages (learning, updating, passing on information)
/// - Memory decay (forgetting low-certainty beliefs over time)
pub trait Agent {
    /// Get this agent's unique identifier.
    fn id(&self) -> AgentId;

    /// Get a reference to this agent's own states.
    fn states(&self) -> &StateMap;

    /// Get a mutable reference to this agent's own states.
    fn states_mut(&mut self) -> &mut StateMap;

    /// Get a reference to this agent's belief store.
    fn beliefs(&self) -> &BeliefStore;

    /// Get a mutable reference to this agent's belief store.
    fn beliefs_mut(&mut self) -> &mut BeliefStore;

    /// Handle an incoming message. Return a list of messages to send in response.
    fn on_message(&mut self, msg: Message) -> Vec<Message>;

    /// Apply memory decay. Default implementation:
    /// - Reduces certainty by 38 per entry
    /// - Degrades external agent sources to Inferred
    /// - Drops zeroed entries
    ///
    /// Override for custom decay strategies.
    fn decay(&mut self) {
        for (_, bmap) in self.beliefs_mut().0.iter_mut() {
            for (_, entries) in bmap.0.iter_mut() {
                for e in entries.iter_mut() {
                    e.certainty = e.certainty.saturating_sub(38);
                    // Source degrades as certainty fades: Agent → Inferred
                    if e.certainty < 102 {
                        if let BeliefSource::Agent(_) = e.source {
                            e.source = BeliefSource::Inferred;
                        }
                    }
                }
                entries.retain(|e| e.certainty > 0);
            }
        }
    }

    /// Report whether a claim from `agent` turned out to be confirmed.
    /// Default: no-op. Override to update trust scores or adjust certainty.
    fn observe_outcome(&mut self, _agent: AgentId, _confirmed: bool) {}

    /// Conflict resolution for incoming beliefs.
    /// Default: keep existing entries that are more certain, append incoming.
    /// Override for trust-weighted merging or semantic reasoning.
    fn resolve_belief(
        &self,
        key: &str,
        from: AgentId,
        existing: Vec<BeliefEntry>,
        incoming: BeliefEntry,
    ) -> Vec<BeliefEntry> {
        let _ = (key, from);
        let incoming_tid = incoming.state.as_any().type_id();
        let incoming_cert = incoming.certainty;
        let mut result: Vec<BeliefEntry> = existing
            .into_iter()
            .filter(|e| {
                !(e.state.as_any().type_id() == incoming_tid && incoming_cert >= e.certainty)
            })
            .collect();
        result.push(incoming);
        result
    }

    /// Merge a BeliefStore payload into this agent's beliefs.
    /// Calls `resolve_belief` for each entry.
    /// Entries originally from `Myself` are never overwritten.
    fn merge_payload(&mut self, from: AgentId, mut payload: BeliefStore) {
        let work: Vec<(String, std::any::TypeId, Vec<BeliefEntry>)> = payload
            .0
            .iter_mut()
            .flat_map(|(key, bmap)| {
                bmap.0
                    .drain()
                    .map(|(tid, entries)| (key.clone(), tid, entries))
                    .collect::<Vec<_>>()
            })
            .collect();

        for (key, tid, incoming_entries) in work {
            for ientry in incoming_entries {
                // Guard: never overwrite self-originated beliefs.
                let protected = self
                    .beliefs()
                    .0
                    .get(&key)
                    .and_then(|bm| bm.0.get(&tid))
                    .map(|v| v.iter().any(|e| matches!(e.source, BeliefSource::Myself)))
                    .unwrap_or(false);

                if protected {
                    continue;
                }

                let existing = self
                    .beliefs_mut()
                    .0
                    .entry(key.clone())
                    .or_insert_with(BeliefMap::new)
                    .0
                    .remove(&tid)
                    .unwrap_or_default();

                let merged = self.resolve_belief(&key, from, existing, ientry);

                self.beliefs_mut()
                    .0
                    .entry(key.clone())
                    .or_insert_with(BeliefMap::new)
                    .0
                    .insert(tid, merged);
            }
        }
    }
}
