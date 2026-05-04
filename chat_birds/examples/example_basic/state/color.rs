use chat_birds::{impl_state, states::StateRepr, verb::Verb, State};

#[derive(Clone)]
pub enum Color {
    Red,
    Green,
    Blue,
}

impl ToString for Color {
    fn to_string(&self) -> String {
        match self {
            Self::Red => "red",
            Self::Green => "green",
            Self::Blue => "blue",
        }
        .into()
    }
}

#[derive(Clone)]
pub struct ColorState {
    color: Color,
}

impl ColorState {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

impl_state!(ColorState);

impl StateRepr for ColorState {
    fn verb(&self) -> Verb {
        Verb::be()
    }

    fn object(&self) -> String {
        self.color.to_string()
    }
}
