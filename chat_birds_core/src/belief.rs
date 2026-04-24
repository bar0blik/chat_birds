use std::any::TypeId;
use std::borrow::Cow;
use std::collections::HashMap;

use crate::core::{AgentId, Probability, State};
use crate::temporal::Temporal;

/// Tracks the origin and degradation path of a belief.
///
/// During decay, sources degrade: Agent(id) → Inferred → entry dropped.
/// Entries from `Myself` are never overridden by external `merge_payload`.
#[derive(Clone, Debug)]
pub enum BeliefSource {
    Myself,
    Agent(AgentId),
    Inferred,
}

/// A single belief entry: a state object with certainty, source, temporal context.
pub struct BeliefEntry {
    pub state: Box<dyn State>,
    pub certainty: u8, // 0..=255
    pub probability: Probability,
    pub source: BeliefSource,
    pub temporal: Temporal,
}

impl BeliefEntry {
    /// Clone this entry, including deep cloning of the state object.
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

/// All belief entries for a single subject (keyed by type).
pub struct BeliefMap(pub HashMap<TypeId, Vec<BeliefEntry>>);

impl BeliefMap {
    pub fn new() -> Self {
        BeliefMap(HashMap::new())
    }

    pub fn insert<S: State + 'static>(&mut self, entry: BeliefEntry) {
        self.0.entry(TypeId::of::<S>()).or_default().push(entry);
    }

    /// Get the highest-certainty entry for state type S. Returns None if no entries exist.
    pub fn get<S: State + 'static>(&self) -> Option<&BeliefEntry> {
        self.0.get(&TypeId::of::<S>()).and_then(|v| {
            v.iter()
                .max_by(|a, b| a.certainty.partial_cmp(&b.certainty).unwrap())
        })
    }

    /// Get all entries for state type S.
    pub fn get_all<S: State + 'static>(&self) -> &[BeliefEntry] {
        self.0
            .get(&TypeId::of::<S>())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Replace all entries for state type S with a new vector.
    pub fn set<S: State + 'static>(&mut self, entries: Vec<BeliefEntry>) {
        self.0.insert(TypeId::of::<S>(), entries);
    }
}

impl Default for BeliefMap {
    fn default() -> Self {
        Self::new()
    }
}

/// A nested belief enables theory of mind: store "what I believe agent X believes" as a State.
///
/// Example:
/// ```ignore
/// my beliefs["agent:1"] → BeliefMap → NestedBelief {
///     store: { "key1" → [BeliefEntry(InBox, certainty=255)] }
/// }
/// ```
/// This means: "I believe agent 1 believes key1 is in a box."
///
/// Nesting is structurally unbounded but agents naturally shallow it by
/// treating deeply nested beliefs with very low certainty.
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

impl Default for NestedBelief {
    fn default() -> Self {
        Self::new()
    }
}

impl State for NestedBelief {
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

/// The complete belief store for an agent: all subjects and their beliefs.
///
/// Maps subject keys (strings) → BeliefMap (type → entries).
/// Subject keys can be agent identifiers ("agent:1"), description keys ("in_box"), etc.
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

impl Default for BeliefStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait to standardize how belief store keys are formatted.
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
