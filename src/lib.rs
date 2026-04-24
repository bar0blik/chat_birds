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
//! - [`belief`]: Belief system (BeliefEntry, BeliefMap, BeliefStore, BeliefKey, NestedBelief)
//! - [`registry`]: Type metadata (StateRegistry for aliases and composites)
//! - [`message`]: Communication (Message, MessageCodec, IntoUtterance, FromUtterance)
//! - [`agent`]: Agent trait and behavior
//! - [`world`]: World trait and simulation management

pub mod agent;
pub mod belief;
pub mod core;
pub mod message;
pub mod registry;
pub mod temporal;
pub mod world;

// Re-export commonly-used types at the crate root for convenience
pub use agent::Agent;
pub use belief::{BeliefEntry, BeliefKey, BeliefMap, BeliefSource, BeliefStore, NestedBelief};
pub use core::{AgentId, Probability, State, StateMap};
pub use message::{FromUtterance, IntoUtterance, Message, MessageCodec};
pub use registry::StateRegistry;
pub use temporal::{Temporal, Tense, Timestamp};
pub use world::World;

// Re-export the impl_state macro
pub use crate::core::{impl_state};
