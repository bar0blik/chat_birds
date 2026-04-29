use crate::BeliefMap;
use crate::{core::State, impl_state};

use std::borrow::Cow;

/// Used to answer "what is [subject]?"
#[derive(Clone)]
pub struct Nature(pub String);

impl_state!(Nature);

/// Used to reference linked states and belief subjects, like possessions or relations.
pub trait RefState: State {
    fn get_ref(&self) -> Cow<'_, str>;
}
