use crate::encoding::{Person, Verb};

pub struct Be;

impl Verb for Be {
    fn first_form(&self) -> String {
        "be".into()
    }
    fn third_form(&self) -> String {
        "been".into()
    }

    fn continuous_form(&self) -> String {
        "being".into()
    }

    fn present_simple(&self, person: Person, plural: bool) -> String {
        match (person, plural) {
            (Person::First, false) => "am".into(),
            (Person::Second, false) | (_, true) => "are".into(),
            (Person::Third, false) => "is".into(),
        }
    }

    fn past_simple(&self, person: Person, plural: bool) -> String {
        match (person, plural) {
            (Person::First, false) | (Person::Third, false) => "was".into(),
            (Person::Second, false) | (_, true) => "were".into(),
        }
    }
}

pub struct Have;

impl Verb for Have {
    fn first_form(&self) -> String {
        "have".into()
    }

    fn second_form(&self) -> String {
        "had".into()
    }

    fn third_form(&self) -> String {
        "had".into()
    }

    fn tperson_form(&self) -> String {
        "has".into()
    }
}
