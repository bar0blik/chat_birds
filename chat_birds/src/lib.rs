//! chat_birds is a lightweight framework for fast, event-driven agent communication.
//!
//! The crate provides the communication structure, while leaving policy decisions to user code.
//! Agents own typed local state, hold beliefs about world subjects, exchange directed messages,
//! and process events through a TTL-guarded message queue.
//!
//! This crate aims to support English-language simulation by assuming any sentence can be expressed
//! as a set of "be" statements the speaker intrinsically understands the relevant
//! states.
//!
//! Core design points:
//! - Typed per-agent state via [`StateMap`] keyed by Rust type.
//! - Belief storage via [`BeliefStore`] keyed by string subjects (including [`AgentId`]).
//! - Directed [`Message`] values with belief payload plus optional natural-language utterance.
//! - User-overridable conflict resolution through [`Agent::resolve_belief`].
//! - Pluggable text encoding/decoding through [`IntoUtterance`] and [`FromUtterance`].

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};

/// Identifier for an agent in the world.
///
/// This is intentionally lightweight and copyable, so it can be used as a routing key
/// in high-frequency message dispatch.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct AgentId(u32);

/// Type-erased state item stored in a [`StateMap`].
///
/// Implementors are user-defined domain types (emotion, position, stamina, etc.) that can be
/// inserted and retrieved by concrete Rust type.
trait State: Any + Send + Sync {
    /// Returns an immutable `Any` view for downcasting.
    fn as_any(&self) -> &dyn Any;
    /// Returns a mutable `Any` view for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Converts boxed state into boxed `Any`.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    /// Clones state behind a trait object.
    ///
    /// This replaces a direct `Clone` bound so trait objects can be duplicated.
    fn clone_box(&self) -> Box<dyn State>;
}

/// Heterogeneous typed storage for an agent's local state.
///
/// Values are keyed by `TypeId`, allowing one value per concrete state type.
struct StateMap(HashMap<TypeId, Box<dyn State>>);

impl StateMap {
    /// Creates an empty state map.
    fn new() -> Self {
        StateMap(HashMap::new())
    }

    /// Inserts or replaces the state value of type `S`.
    fn insert<S: State + 'static>(&mut self, s: S) {
        self.0.insert(TypeId::of::<S>(), Box::new(s));
    }

    /// Gets a shared reference to state of type `S`.
    fn get<S: State + 'static>(&self) -> Option<&S> {
        self.0
            .get(&TypeId::of::<S>())
            .and_then(|boxed| boxed.as_any().downcast_ref::<S>())
    }

    /// Gets a mutable reference to state of type `S`.
    fn get_mut<S: State + 'static>(&mut self) -> Option<&mut S> {
        self.0
            .get_mut(&TypeId::of::<S>())
            .and_then(|boxed| boxed.as_any_mut().downcast_mut::<S>())
    }

    /// Removes and returns state of type `S`, if present.
    fn remove<S: State + 'static>(&mut self) -> Option<Box<dyn State>> {
        self.0.remove(&TypeId::of::<S>())
    }
}

/// A key that identifies a belief subject in a [`BeliefStore`].
///
/// The crate supports both semantic string keys (for world facts) and [`AgentId`]
/// keys (for agent-centric beliefs).
trait BeliefKey {
    /// Converts this key into a canonical string representation.
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

/// Beliefs grouped by subject key, each subject holding a typed [`StateMap`].
///
/// Typical keys are world labels (for example, `"weather"`) or agent-scoped keys
/// derived from [`AgentId`].
struct BeliefStore(HashMap<String, StateMap>);

impl BeliefStore {
    /// Creates an empty belief store.
    fn new() -> Self {
        BeliefStore(HashMap::new())
    }

    /// Returns beliefs for a subject key.
    fn get(&self, key: &impl BeliefKey) -> Option<&StateMap> {
        self.0.get(key.to_key().as_ref())
    }

    /// Returns mutable beliefs for a subject key.
    fn get_mut(&mut self, key: &impl BeliefKey) -> Option<&mut StateMap> {
        self.0.get_mut(key.to_key().as_ref())
    }

    /// Inserts or replaces the full state map for a subject key.
    fn insert(&mut self, key: &impl BeliefKey, states: StateMap) {
        self.0.insert(key.to_key().into_owned(), states);
    }
}

/// An actor that owns local state, world beliefs, and message handling behavior.
///
/// Implementors define policy: how to react to incoming messages, how to merge beliefs,
/// and how to resolve conflicts between old and incoming evidence.
trait Agent {
    /// Returns this agent's stable identifier.
    fn id(&self) -> AgentId;

    /// Returns immutable access to local typed state.
    fn states(&self) -> &StateMap;

    /// Returns mutable access to local typed state.
    fn states_mut(&mut self) -> &mut StateMap;

    /// Returns immutable access to the belief store.
    fn beliefs(&self) -> &BeliefStore;

    /// Returns mutable access to the belief store.
    fn beliefs_mut(&mut self) -> &mut BeliefStore;

    /// Handles one incoming message and emits zero or more response messages.
    ///
    /// Event flow: receive message, update beliefs, then produce follow-up messages.
    fn on_message(&mut self, msg: Message) -> Vec<Message>;

    /// Merges message payload beliefs into this agent's belief store.
    ///
    /// Implementors can override this to define merge strategy. During merge, conflicts can be
    /// delegated to [`Self::resolve_belief`].
    fn merge_payload(&mut self, msg: &Message) {
        // TODO: loop over BeliefStore and update own StateMap using resolve_belief
    }

    /// Resolves a conflict between an old and incoming belief state.
    ///
    /// The default strategy overwrites with `new`. Override to implement trust-weighting,
    /// source reliability scoring, temporal decay, or other game-specific policy.
    fn resolve_belief(
        &self,
        key: &str,
        from: AgentId,
        old: &dyn State,
        new: &dyn State,
    ) -> Box<dyn State> {
        new.clone_box()
    }
}

/// Encodes a type into a text utterance.
///
/// This trait decouples text representation from core agent/world mechanics.
trait IntoUtterance {
    /// Converts this value into a serialized utterance string.
    fn to_utterance(&self) -> String;
}

/// Decodes a type from a text utterance.
///
/// Return `None` when parsing fails or input is semantically invalid.
trait FromUtterance: Sized {
    /// Attempts to parse a value from an utterance.
    fn from_utterance(s: &str) -> Option<Self>;
}

/// Directed message sent from one agent to another.
///
/// Messages can carry both structured belief updates (`payload`) and optional free-form
/// text (`utterance`).
struct Message {
    /// Sender agent id.
    pub from: AgentId,
    /// Recipient agent id.
    pub to: AgentId,
    /// Beliefs communicated by the sender.
    pub payload: BeliefStore,
    /// Optional text channel representation.
    pub utterance: Option<String>,
    /// Remaining hop budget for queue dispatch.
    pub ttl: u8,
}

/// Placeholder text encoder for [`Message`].
///
/// Replace this with application-specific serialization.
impl IntoUtterance for Message {
    fn to_utterance(&self) -> String {
        "Placeholder".into()
    }
}

/// Placeholder text decoder for [`Message`].
///
/// Replace this with application-specific parsing.
impl FromUtterance for Message {
    fn from_utterance(s: &str) -> Option<Self> {
        None
    }
}

/// Environment that stores agents and dispatches queued messages.
///
/// The default dispatcher performs directed delivery and decrements message TTL each hop,
/// preventing unbounded loops in cyclic response graphs.
trait World {
    /// Returns immutable access to the world agent map.
    fn agents(&self) -> &HashMap<AgentId, Box<dyn Agent>>;

    /// Returns mutable access to the world agent map.
    fn agents_mut(&mut self) -> &mut HashMap<AgentId, Box<dyn Agent>>;

    /// Dispatches an initial message and processes the queue until empty.
    ///
    /// Behavior:
    /// - Drops messages targeting unknown agents.
    /// - Drops messages whose `ttl` is already zero.
    /// - Decrements TTL before invoking recipient handlers.
    /// - Enqueues all responses emitted by handlers.
    fn dispatch(&mut self, initial: Message) {
        let mut queue = VecDeque::new();
        queue.push_back(initial);

        while let Some(mut msg) = queue.pop_front() {
            let Some(recipient) = self.agents_mut().get_mut(&msg.to) else {
                continue;
            };

            if msg.ttl == 0 {
                continue;
            }

            msg.ttl -= 1;

            let responses = recipient.on_message(msg);
            queue.extend(responses);
        }
    }
}
