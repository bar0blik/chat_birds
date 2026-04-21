#![allow(dead_code)]

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
//  CHAT_BIRDS CORE
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AgentId(pub u32);

// ── State ─────────────────────────────────────────────────────────────────────

pub trait State: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn clone_box(&self) -> Box<dyn State>;
}

#[macro_export]
macro_rules! impl_state {
    ($t:ty) => {
        impl State for $t {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
            fn clone_box(&self) -> Box<dyn State> {
                Box::new(self.clone())
            }
        }
    };
}

// ── StateMap (own, always-certain states) ─────────────────────────────────────

pub struct StateMap(pub HashMap<TypeId, Box<dyn State>>);

impl StateMap {
    pub fn new() -> Self {
        StateMap(HashMap::new())
    }

    pub fn insert<S: State + 'static>(&mut self, s: S) {
        self.0.insert(TypeId::of::<S>(), Box::new(s));
    }

    pub fn get<S: State + 'static>(&self) -> Option<&S> {
        self.0
            .get(&TypeId::of::<S>())
            .and_then(|b| b.as_any().downcast_ref::<S>())
    }

    pub fn get_mut<S: State + 'static>(&mut self) -> Option<&mut S> {
        self.0
            .get_mut(&TypeId::of::<S>())
            .and_then(|b| b.as_any_mut().downcast_mut::<S>())
    }

    pub fn remove<S: State + 'static>(&mut self) -> Option<Box<dyn State>> {
        self.0.remove(&TypeId::of::<S>())
    }

    pub fn remove_as<S: State + 'static>(&mut self) -> Option<S> {
        self.0
            .remove(&TypeId::of::<S>())
            .and_then(|b| b.into_any().downcast::<S>().ok())
            .map(|b| *b)
    }
}

// ── Probability ───────────────────────────────────────────────────────────────
//
// Condition(key): holds if the belief at `key` is present and certain enough.
// Resolved externally by the user at query time.

#[derive(Clone, Debug)]
pub enum Probability {
    Level(u8),         // 0 = impossible, 255 = certain
    Condition(String), // belief-store subject key that must hold
    Always,
    Never,
}

// ── Temporal ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum Tense {
    Past,
    Present,
    Future,
}

#[derive(Clone, Debug)]
pub enum Temporal {
    Clock { hour: u8, minute: u8 },
    Tense(Tense),
    Period { start: u64, end: u64 }, // game ticks
    Always,
}

// ── BeliefSource ──────────────────────────────────────────────────────────────
//
// Degrades during decay: Agent(id) → Inferred → entry dropped.
// Entries from Myself are never overridden by external merge_payload.

#[derive(Clone, Debug)]
pub enum BeliefSource {
    Myself,
    Agent(AgentId),
    Inferred,
}

// ── BeliefEntry ───────────────────────────────────────────────────────────────

pub struct BeliefEntry {
    pub state: Box<dyn State>,
    pub certainty: u8, // 0..=255
    pub probability: Probability,
    pub source: BeliefSource,
    pub temporal: Temporal,
}

impl BeliefEntry {
    pub fn clone_entry(&self) -> BeliefEntry {
        BeliefEntry {
            state: self.state.clone_box(),
            certainty: self.certainty,
            probability: self.probability.clone(),
            source: self.source.clone(),
            temporal: self.temporal.clone(),
        }
    }
}

// ── BeliefMap (all entries for one subject) ───────────────────────────────────

pub struct BeliefMap(pub HashMap<TypeId, Vec<BeliefEntry>>);

impl BeliefMap {
    pub fn new() -> Self {
        BeliefMap(HashMap::new())
    }

    pub fn insert<S: State + 'static>(&mut self, entry: BeliefEntry) {
        self.0.entry(TypeId::of::<S>()).or_default().push(entry);
    }

    pub fn get<S: State + 'static>(&self) -> Option<&BeliefEntry> {
        self.0.get(&TypeId::of::<S>()).and_then(|v| {
            v.iter()
                .max_by(|a, b| a.certainty.partial_cmp(&b.certainty).unwrap())
        })
    }

    pub fn get_all<S: State + 'static>(&self) -> &[BeliefEntry] {
        self.0
            .get(&TypeId::of::<S>())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn set<S: State + 'static>(&mut self, entries: Vec<BeliefEntry>) {
        self.0.insert(TypeId::of::<S>(), entries);
    }
}

// ── NestedBelief ──────────────────────────────────────────────────────────────
//
// Enables theory of mind: store "what I believe agent X believes" as a State.
//
// Example:
//   my beliefs["agent:1"] → BeliefMap → NestedBelief {
//       store: { "key1" → [BeliefEntry(InBox, certainty=255)] }
//   }
// Meaning: "I believe agent 1 believes key1 is in a box."
//
// Nesting is unbounded in structure but agents naturally shallow this by
// treating deeply nested beliefs with very low certainty.

#[derive(Clone)]
pub struct NestedBelief {
    pub store: BeliefStore,
}

impl NestedBelief {
    pub fn new() -> Self {
        NestedBelief {
            store: BeliefStore::new(),
        }
    }
}

impl State for NestedBelief {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn clone_box(&self) -> Box<dyn State> {
        Box::new(self.clone())
    }
}

// ── BeliefStore ───────────────────────────────────────────────────────────────

pub struct BeliefStore(pub HashMap<String, BeliefMap>);

impl BeliefStore {
    pub fn new() -> Self {
        BeliefStore(HashMap::new())
    }

    pub fn get(&self, key: &impl BeliefKey) -> Option<&BeliefMap> {
        self.0.get(key.to_key().as_ref())
    }

    pub fn get_mut(&mut self, key: &impl BeliefKey) -> Option<&mut BeliefMap> {
        self.0.get_mut(key.to_key().as_ref())
    }

    pub fn get_or_insert(&mut self, key: &impl BeliefKey) -> &mut BeliefMap {
        self.0
            .entry(key.to_key().into_owned())
            .or_insert_with(BeliefMap::new)
    }
}

impl Clone for BeliefStore {
    fn clone(&self) -> Self {
        let mut map = HashMap::new();
        for (key, bmap) in &self.0 {
            let mut new_bmap = BeliefMap::new();
            for (tid, entries) in &bmap.0 {
                new_bmap
                    .0
                    .insert(*tid, entries.iter().map(|e| e.clone_entry()).collect());
            }
            map.insert(key.clone(), new_bmap);
        }
        BeliefStore(map)
    }
}

// ── BeliefKey ─────────────────────────────────────────────────────────────────

pub trait BeliefKey {
    fn to_key(&self) -> Cow<'_, str>;
}

impl BeliefKey for AgentId {
    fn to_key(&self) -> Cow<'_, str> {
        Cow::Owned(format!("agent:{}", self.0))
    }
}

impl<'a> BeliefKey for &'a str {
    fn to_key(&self) -> Cow<'_, str> {
        Cow::Borrowed(self)
    }
}

impl BeliefKey for String {
    fn to_key(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.as_str())
    }
}

impl<T: BeliefKey + ?Sized> BeliefKey for &T {
    fn to_key(&self) -> Cow<'_, str> {
        (*self).to_key()
    }
}

// ── StateRegistry ─────────────────────────────────────────────────────────────
//
// Tracks semantic relationships between state types:
//   alias:     TypeId A → TypeId B  ("hungry" is another word for "needs_food")
//   composite: TypeId A → [TypeId]  ("starving" = high hunger AND low health)
//
// Used during belief merging to detect overlap and during display.
// Does not enforce merging automatically — the Agent decides.

pub struct StateRegistry {
    aliases: HashMap<TypeId, TypeId>,
    composites: HashMap<TypeId, Vec<TypeId>>,
    labels: HashMap<TypeId, &'static str>,
}

impl StateRegistry {
    pub fn new() -> Self {
        StateRegistry {
            aliases: HashMap::new(),
            composites: HashMap::new(),
            labels: HashMap::new(),
        }
    }

    pub fn register<S: State + 'static>(&mut self, label: &'static str) {
        self.labels.insert(TypeId::of::<S>(), label);
    }

    /// Declare A is an alias for B. During resolution, an A entry may be
    /// semantically merged with a B entry.
    pub fn alias<A: State + 'static, B: State + 'static>(&mut self) {
        self.aliases.insert(TypeId::of::<A>(), TypeId::of::<B>());
    }

    pub fn composite<A: State + 'static>(&mut self, components: Vec<TypeId>) {
        self.composites.insert(TypeId::of::<A>(), components);
    }

    pub fn canonical(&self, tid: TypeId) -> TypeId {
        *self.aliases.get(&tid).unwrap_or(&tid)
    }

    pub fn label(&self, tid: TypeId) -> Option<&'static str> {
        self.labels.get(&tid).copied()
    }
}

// ── Message ───────────────────────────────────────────────────────────────────

pub struct Message {
    pub from: AgentId,
    pub to: AgentId,
    pub payload: BeliefStore,
    pub utterance: Option<String>,
    pub ttl: u8,
}

// ── Utterance ─────────────────────────────────────────────────────────────────

pub trait IntoUtterance {
    fn to_utterance(&self) -> String;
}

pub trait FromUtterance: Sized {
    fn from_utterance(s: &str) -> Option<Self>;
}

// ── Agent ─────────────────────────────────────────────────────────────────────

pub trait Agent {
    fn id(&self) -> AgentId;
    fn states(&self) -> &StateMap;
    fn states_mut(&mut self) -> &mut StateMap;
    fn beliefs(&self) -> &BeliefStore;
    fn beliefs_mut(&mut self) -> &mut BeliefStore;

    fn on_message(&mut self, msg: Message) -> Vec<Message>;

    /// Apply memory decay. Default: -38 certainty per entry, degrade sources,
    /// drop zeroes. Override for custom decay strategies.
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

    /// Report whether a claim from `agent` turned out to be correct.
    /// Default: no-op. Override to update trust scores.
    fn observe_outcome(&mut self, _agent: AgentId, _confirmed: bool) {}

    /// Conflict resolution. Default: append incoming, drop lower-certainty
    /// same-type entries. Override for trust-weighted merging.
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

    /// Merge a BeliefStore payload into self.beliefs, calling resolve_belief
    /// for each entry. Entries originally from Myself are never overwritten.
    fn merge_payload(&mut self, from: AgentId, mut payload: BeliefStore) {
        let work: Vec<(String, TypeId, Vec<BeliefEntry>)> = payload
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

// ── World ─────────────────────────────────────────────────────────────────────

pub trait World {
    fn agents(&self) -> &HashMap<AgentId, Box<dyn Agent>>;
    fn agents_mut(&mut self) -> &mut HashMap<AgentId, Box<dyn Agent>>;

    fn dispatch(&mut self, initial: Message) {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(initial);
        while let Some(mut msg) = queue.pop_front() {
            if msg.ttl == 0 {
                continue;
            }
            msg.ttl -= 1;
            let Some(recipient) = self.agents_mut().get_mut(&msg.to) else {
                continue;
            };
            let responses = recipient.on_message(msg);
            queue.extend(responses);
        }
    }
}
