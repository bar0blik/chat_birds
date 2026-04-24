use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Agent identifier (u16 for memory efficiency).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AgentId(pub u16);

/// Type erasure trait for storing arbitrary state objects with cloning support.
pub trait State: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn clone_box(&self) -> Box<dyn State>;
}

/// Generates a State trait implementation for any Clone type.
/// Eliminates boilerplate in agent state implementations.
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

/// Storage for an agent's own state objects (not beliefs, but direct states).
/// Uses TypeId for runtime type discrimination.
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

impl Default for StateMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents degrees of certainty or probabilistic knowledge.
#[derive(Clone, Debug)]
pub enum Probability {
    Level(u8),         // 0 = impossible, 255 = certain
    Condition(String), // belief-store subject key that must hold
    Always,
    Never,
}
