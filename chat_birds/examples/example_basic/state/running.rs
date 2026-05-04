use chat_birds::{impl_state, states::StateRepr, verb::Verb, State};

#[derive(Clone)]
pub struct RunningState;

impl_state!(RunningState);

impl StateRepr for RunningState {
    fn verb(&self) -> Verb {
        Verb::SemiIrregular {
            base: "run".into(),
            past: "ran".into(),
            past_participle: None,
            present_participle: None,
            third_person: None,
        }
    }

    fn object(&self) -> String {
        "".into()
    }
}
