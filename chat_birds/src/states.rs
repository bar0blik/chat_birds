use std::any::Any;

use crate::verb::Verb;
use crate::State;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::RwLock;

fn short_type_name(full: &str) -> &str {
    full.rsplit("::").next().unwrap_or(full)
}

pub trait StateRepr: State + Any {
    fn object(&self) -> String {
        short_type_name(std::any::type_name_of_val(self)).to_lowercase()
    }

    fn verb(&self) -> Verb {
        Verb::be()
    }
}

/// Extension trait to bridge `State` → `StateRepr` without modifying core.
/// Implemented automatically for any type that implements both traits.
pub trait StateAsRepr: State {
    fn as_state_repr(&self) -> Option<&dyn StateRepr>;
}

// Auto-implement for any concrete type T that implements State + StateRepr
impl<T> StateAsRepr for T
where
    T: State + StateRepr + 'static,
{
    fn as_state_repr(&self) -> Option<&dyn StateRepr> {
        Some(self) // coercion to &dyn StateRepr works automatically
    }
}

type ReprFn = fn(&dyn State) -> Option<&dyn StateRepr>;

static REGISTRY: OnceLock<RwLock<HashMap<TypeId, ReprFn>>> = OnceLock::new();

/// Register a concrete `StateRepr` implementation so trait-object lookups work.
pub fn register_state_repr<T: StateRepr + 'static>() {
    let map = REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
    let mut write = map.write().unwrap();
    let f: ReprFn = |s: &dyn State| s.as_any().downcast_ref::<T>().map(|t| t as &dyn StateRepr);
    write.insert(TypeId::of::<T>(), f);
}

/// Allow calling `as_state_repr()` on trait objects (`dyn State`) by
/// consulting the registry populated via `register_state_repr::<T>()`.
impl StateAsRepr for dyn State {
    fn as_state_repr(&self) -> Option<&dyn StateRepr> {
        let map = REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
        let read = map.read().unwrap();
        read.get(&self.as_any().type_id()).and_then(|f| f(self))
    }
}

/// Convenience macro to call `register_state_repr` from user code.
#[macro_export]
macro_rules! register_state_repr {
    ($t:ty) => {
        $crate::states::register_state_repr::<$t>();
    };
}
