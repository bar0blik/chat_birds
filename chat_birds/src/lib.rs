use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};

// ── Macros ────────────────────────────────────────────────────────────────────

/// Implements the `State` trait for a type that derives `Clone`.
///
/// Eliminates boilerplate for the four required methods: `as_any`, `as_any_mut`,
/// `into_any`, and `clone_box`.
///
/// # Example
/// ```ignore
/// #[derive(Clone)]
/// struct Emotion(String);
///
/// impl_state!(Emotion);
/// ```
#[macro_export]
macro_rules! impl_state {
    ($ty:ty) => {
        impl $crate::State for $ty {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
            fn clone_box(&self) -> Box<dyn $crate::State> {
                Box::new(self.clone())
            }
        }
    };
}

// ── AgentId ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AgentId(pub u32);

// ── State ─────────────────────────────────────────────────────────────────────

pub trait State: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn clone_box(&self) -> Box<dyn State>;
}

// ── StateMap (own states — always certain, source = self) ─────────────────────

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

pub enum Probability {
    Level(u8),                 // 0 = impossible, 255 = certain
    Condition(Box<dyn State>), // true if state holds
    Always,
    Never,
}

// ── Temporal ──────────────────────────────────────────────────────────────────

pub enum Tense {
    Past,
    Present,
    Future,
}

pub enum Temporal {
    Clock { hour: u8, minute: u8 },
    Tense(Tense),
    Period { start: u64, end: u64 }, // game ticks
    Always,
}

// ── BeliefSource ──────────────────────────────────────────────────────────────

pub enum BeliefSource {
    Myself,
    Agent(AgentId),
    Inferred,
}

// ── BeliefEntry ───────────────────────────────────────────────────────────────

pub struct BeliefEntry {
    pub state: Box<dyn State>,
    pub certainty: f32, // 0.0..=1.0
    pub probability: Probability,
    pub source: BeliefSource,
    pub temporal: Temporal,
}

// ── BeliefMap (beliefs about one subject) ─────────────────────────────────────

pub struct BeliefMap(pub HashMap<TypeId, Vec<BeliefEntry>>);

impl BeliefMap {
    pub fn new() -> Self {
        BeliefMap(HashMap::new())
    }

    pub fn insert<S: State + 'static>(&mut self, entry: BeliefEntry) {
        self.0.entry(TypeId::of::<S>()).or_default().push(entry);
    }

    /// Returns highest-certainty entry for type S.
    pub fn get<S: State + 'static>(&self) -> Option<&BeliefEntry> {
        self.0.get(&TypeId::of::<S>()).and_then(|v| {
            v.iter()
                .max_by(|a, b| a.certainty.partial_cmp(&b.certainty).unwrap())
        })
    }

    /// All interpretations for type S.
    pub fn get_all<S: State + 'static>(&self) -> &[BeliefEntry] {
        self.0
            .get(&TypeId::of::<S>())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Replaces all entries for type S — used by resolve_belief output.
    pub fn set<S: State + 'static>(&mut self, entries: Vec<BeliefEntry>) {
        self.0.insert(TypeId::of::<S>(), entries);
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

impl BeliefKey for str {
    fn to_key(&self) -> Cow<'_, str> {
        Cow::Borrowed(self)
    }
}

impl BeliefKey for String {
    fn to_key(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.as_str())
    }
}

/// Blanket impl to allow references to BeliefKey types.
impl<T: BeliefKey + ?Sized> BeliefKey for &T {
    fn to_key(&self) -> Cow<'_, str> {
        (*self).to_key()
    }
}

// ── BeliefStore (beliefs about all subjects) ──────────────────────────────────

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

// ── Message ───────────────────────────────────────────────────────────────────

pub struct Message {
    pub from: AgentId,
    pub to: AgentId,
    pub payload: BeliefStore, // slice of sender's beliefs
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

    /// Own states — fully certain, not overridable by external messages.
    fn states(&self) -> &StateMap;
    fn states_mut(&mut self) -> &mut StateMap;

    /// Beliefs about other agents and world entities.
    fn beliefs(&self) -> &BeliefStore;
    fn beliefs_mut(&mut self) -> &mut BeliefStore;

    fn on_message(&mut self, msg: Message) -> Vec<Message>;

    /// User-triggered: implement own decay logic (certainty drop, source loss, etc.)
    fn decay(&mut self);

    /// Conflict resolution. Receives existing entries (moved) + incoming entry.
    /// Default: append → keeps all interpretations. Override for trust-weighted merging.
    fn resolve_belief(
        &self,
        key: &str,
        from: AgentId,
        existing: Vec<BeliefEntry>,
        incoming: BeliefEntry,
    ) -> Vec<BeliefEntry> {
        let mut entries = existing;
        entries.push(incoming);
        entries
    }

    /// Merges a payload BeliefStore into self.beliefs, calling resolve_belief
    /// per entry so trust-weighted conflict resolution is applied automatically.
    fn merge_payload(&mut self, from: AgentId, mut payload: BeliefStore) {
        // Collect work upfront to avoid holding borrows during resolution.
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
                // Scoped mut borrow — released before resolve_belief's shared borrow.
                let existing = {
                    self.beliefs_mut()
                        .0
                        .entry(key.clone())
                        .or_insert_with(BeliefMap::new)
                        .0
                        .remove(&tid)
                        .unwrap_or_default()
                };

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
        let mut queue = VecDeque::new();
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
