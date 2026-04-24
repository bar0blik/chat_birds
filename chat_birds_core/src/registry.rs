use std::any::TypeId;
use std::collections::HashMap;

use crate::core::State;

/// Tracks semantic relationships between state types.
///
/// Used during belief merging to detect overlap and during display.
/// Does not enforce merging automatically — the Agent decides.
///
/// Features:
/// - **alias**: TypeId A → TypeId B ("hungry" is another word for "needs_food")
/// - **composite**: TypeId A → [TypeId] ("starving" = high hunger AND low health)
/// - **label**: Friendly names for debugging and display
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

    /// Register a human-readable label for a state type.
    pub fn register<S: State + 'static>(&mut self, label: &'static str) {
        self.labels.insert(TypeId::of::<S>(), label);
    }

    /// Declare that type A is an alias for type B.
    /// During resolution, an A entry may be semantically merged with a B entry.
    pub fn alias<A: State + 'static, B: State + 'static>(&mut self) {
        self.aliases.insert(TypeId::of::<A>(), TypeId::of::<B>());
    }

    /// Declare that type A is a composite of the given component types.
    pub fn composite<A: State + 'static>(&mut self, components: Vec<TypeId>) {
        self.composites.insert(TypeId::of::<A>(), components);
    }

    /// Resolve a type ID to its canonical form (following alias chain).
    pub fn canonical(&self, tid: TypeId) -> TypeId {
        *self.aliases.get(&tid).unwrap_or(&tid)
    }

    /// Look up the label for a type ID.
    pub fn label(&self, tid: TypeId) -> Option<&'static str> {
        self.labels.get(&tid).copied()
    }
}

impl Default for StateRegistry {
    fn default() -> Self {
        Self::new()
    }
}
