use chat_birds::{impl_state, states::{StateRepr, AdjectiveCategory}, verb::Verb, State, ObjectFragment};

#[derive(Clone)]
pub enum Feeling {
    Good,
    Fine,
    Bad,
}

impl ToString for Feeling {
    fn to_string(&self) -> String {
        match self {
            Self::Good => "good",
            Self::Fine => "fine",
            Self::Bad => "bad",
        }
        .into()
    }
}

#[derive(Clone)]
pub struct FeelingState {
    feeling: Feeling,
}

impl FeelingState {
    pub fn new(feeling: Feeling) -> Self {
        FeelingState { feeling }
    }
}

impl_state!(FeelingState);

impl StateRepr for FeelingState {
    fn verb(&self) -> Verb {
        // Feel
        Verb::SemiIrregular {
            base: "feel".into(),
            past: "felt".into(),
            past_participle: None,
            present_participle: None,
            third_person: None,
        }
    }

    fn object(&self) -> Option<ObjectFragment> {
        Some(ObjectFragment::adjective(
            self.feeling.to_string(),
            AdjectiveCategory::Opinion,
        ))
    }
}
