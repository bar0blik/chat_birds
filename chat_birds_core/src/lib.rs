#![allow(dead_code)]

//! # chat_birds: A belief/agent system for game world simulation
//!
//! A compact, memory-efficient framework for simulating agent beliefs, knowledge propagation,
//! and decision-making in game worlds. Agents can hold uncertain beliefs about the world,
//! exchange information via messages, and update their knowledge over time.
//!
//! ## Core Concepts
//!
//! - **State**: Type-erased, cloneable objects representing direct facts about an agent.
//! - **Belief**: A probabilistic hypothesis about the world with certainty, source, and temporal context.
//! - **Agent**: An autonomous entity with states and beliefs that can receive and send messages.
//! - **World**: A container for agents that manages message routing and dispatch.
//!
//! ## Module Organization
//!
//! - [`core`]: Core types (AgentId, State trait, StateMap, Probability)
//! - [`temporal`]: Temporal representation (Tense, Timestamp with compact 64-bit encoding, Temporal)
//! - [`belief`]: Belief system (Belief, BeliefMap, BeliefStore, BeliefKey, NestedBelief)
//! - [`source`]: Sourcing system (SourceMap, Trust)
//! - [`registry`]: Type metadata (StateRegistry for aliases and composites)
//! - [`message`]: Communication (Message, MessageCodec, IntoUtterance, FromUtterance)
//! - [`agent`]: Agent trait and behavior
//! - [`world`]: World trait and simulation management
//! - [`identity`]: Indentity handling (Nature, RefState)

pub mod agent;
pub mod belief;
pub mod core;
pub mod identity;
pub mod message;
pub mod registry;
pub mod source;
pub mod temporal;
pub mod world;

// Re-export commonly-used types at the crate root for convenience
pub use agent::Agent;
pub use belief::{
    Belief, BeliefKey, BeliefMap, BeliefSet, BeliefSource, BeliefStore, NestedBelief,
    SubjectBeliefs,
};
pub use core::{AgentId, Probability, State, StateMap};
pub use identity::{Nature, RefState};
pub use message::{FromUtterance, IntoUtterance, Message, MessageCodec};
pub use registry::StateRegistry;
pub use source::{SourceMap, Trust};
pub use temporal::{Temporal, TenseTime, Timestamp};
pub use world::World;
