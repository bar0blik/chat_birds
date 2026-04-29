use std::any::Any;

use crate::verb::{verbs::Be, Verb};
use crate::State;

fn short_type_name(full: &str) -> &str {
    full.rsplit("::").next().unwrap_or(full)
}

pub trait StateRepr: State + Any {
    fn object(&self) -> String {
        short_type_name(std::any::type_name_of_val(self)).to_lowercase()
    }

    fn verb(&self) -> impl Verb {
        Be
    }
}
