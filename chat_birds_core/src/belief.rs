//! Agent holds a BeliefStore
//! A BeliefStore maps each string key (subject) to a SubjectBeliefs
//! A SubjectBeliefs holds info about the subject (plurality) and its BeliefMap
//! A BeliefMap maps a State's TypeId to a BeliefSet
//! A BeliefEntryVec is a SmallVec of BeliefEntry
//! A BeliefEntry stores the actual State and the certainty, probability, source and the temporality of the State
use smallvec::SmallVec;
use std::any::TypeId;
use std::borrow::Cow;
use std::collections::hash_map::{IntoIter, Iter, IterMut};
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

/// A single belief: a state object with certainty, source, temporal context.
pub struct Belief {
    pub state: Box<dyn State>,
    pub certainty: u8, // 0..=255
    pub probability: Probability,
    pub source: BeliefSource,
    pub temporal: Temporal,
}

impl Clone for Belief {
    /// Clone this belief, including deep cloning of the state object.
    fn clone(&self) -> Belief {
        Belief {
            state: self.state.clone_box(),
            certainty: self.certainty,
            probability: self.probability.clone(),
            source: self.source.clone(),
            temporal: self.temporal.clone(),
        }
    }
}

/// Collection of belief entries
/// ```ignore
/// pub type BeliefSet = SmallVec<[Belief; 1]>;
/// ```
pub type BeliefSet = SmallVec<[Belief; 1]>;

/// Query parameters to search a belief map
pub struct QueryParam;

/// All belief entries for a single subject (keyed by type).
#[derive(Clone)]
pub struct BeliefMap(pub HashMap<TypeId, BeliefSet>);

impl BeliefMap {
    pub fn new() -> Self {
        BeliefMap(HashMap::new())
    }

    pub fn iter(&self) -> Iter<'_, TypeId, BeliefSet> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, TypeId, BeliefSet> {
        self.0.iter_mut()
    }

    pub fn insert<S: State + 'static>(&mut self, entry: Belief) {
        self.0.entry(TypeId::of::<S>()).or_default().push(entry);
    }

    /// Get the highest-certainty entry for state type S. Returns None if no entries exist.
    pub fn query<S: State + 'static>(&self, _param: QueryParam) -> Vec<&Belief> {
        Vec::new()
    }

    /// Get all entries for state type S.
    pub fn get_vec<S: State + 'static>(&self) -> Option<&BeliefSet> {
        self.0.get(&TypeId::of::<S>())
    }

    /// Get all entries as mutable for state type S.
    pub fn get_vec_mut<S: State + 'static>(&mut self) -> Option<&mut BeliefSet> {
        self.0.get_mut(&TypeId::of::<S>())
    }
}

impl Default for BeliefMap {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for BeliefMap {
    type Item = (TypeId, BeliefSet);
    type IntoIter = IntoIter<TypeId, BeliefSet>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a BeliefMap {
    type Item = (&'a TypeId, &'a BeliefSet);
    type IntoIter = Iter<'a, TypeId, BeliefSet>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut BeliefMap {
    type Item = (&'a TypeId, &'a mut BeliefSet);
    type IntoIter = IterMut<'a, TypeId, BeliefSet>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
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

#[derive(Clone)]
pub struct SubjectBeliefs {
    pub plural: bool,
    pub map: BeliefMap,
}

impl SubjectBeliefs {
    pub fn new() -> Self {
        Self {
            plural: false,
            map: BeliefMap::new(),
        }
    }
}

/// The complete belief store for an agent: all subjects and their beliefs.
///
/// Maps subject keys (strings) → (plural flag, BeliefMap (type → entries)).
/// Subject keys can be agent identifiers ("agent:1"), description keys ("in_box"), etc.
#[derive(Clone)]
pub struct BeliefStore(pub HashMap<String, SubjectBeliefs>);

impl BeliefStore {
    pub fn new() -> Self {
        BeliefStore(HashMap::new())
    }

    pub fn get(&self, key: &impl BeliefKey) -> Option<&SubjectBeliefs> {
        self.0.get(key.to_key().as_ref())
    }

    pub fn get_mut(&mut self, key: &impl BeliefKey) -> Option<&mut SubjectBeliefs> {
        self.0.get_mut(key.to_key().as_ref())
    }

    pub fn get_or_insert(&mut self, key: &impl BeliefKey) -> &mut SubjectBeliefs {
        self.0
            .entry(key.to_key().into_owned())
            .or_insert_with(SubjectBeliefs::new)
    }

    pub fn insert(
        &mut self,
        key: &impl BeliefKey,
        value: SubjectBeliefs,
    ) -> Option<SubjectBeliefs> {
        self.0.insert(key.to_key().into_owned(), value)
    }
}

impl Default for BeliefStore {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for BeliefStore {
    type Item = (String, SubjectBeliefs);
    type IntoIter = IntoIter<String, SubjectBeliefs>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a BeliefStore {
    type Item = (&'a String, &'a SubjectBeliefs);
    type IntoIter = Iter<'a, String, SubjectBeliefs>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut BeliefStore {
    type Item = (&'a String, &'a mut SubjectBeliefs);
    type IntoIter = IterMut<'a, String, SubjectBeliefs>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
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
