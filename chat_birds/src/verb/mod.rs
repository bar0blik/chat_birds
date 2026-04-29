pub mod verbs;

pub mod person;
pub use person::{InvalidPerson, Person};

use chat_birds_core::TenseTime;

use crate::verb::verbs::{Be, Have};

const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];

fn is_vowel(c: char) -> bool {
    VOWELS.contains(&c)
}

pub enum TenseGroup {
    Simple,
    Continuous,
    Perfect,
    PerfectContinuous,
}

pub struct Tense {
    pub(crate) time: TenseTime,
    pub(crate) group: TenseGroup,
}

// TODO?: maybe replace the trait with an enum
pub trait Verb {
    fn get(&self, person: Person, plural: bool, tense: Tense) -> String {
        match (tense.time, tense.group) {
            (TenseTime::Present, TenseGroup::Simple) => self.present_simple(person, plural),
            (TenseTime::Present, TenseGroup::Continuous) => self.present_continuous(person, plural),
            (TenseTime::Present, TenseGroup::Perfect) => self.present_perfect(person, plural),
            (TenseTime::Present, TenseGroup::PerfectContinuous) => {
                self.present_perfect_continuous(person, plural)
            }

            (TenseTime::Past, TenseGroup::Simple) => self.past_simple(person, plural),
            (TenseTime::Past, TenseGroup::Continuous) => self.past_continuous(person, plural),
            (TenseTime::Past, TenseGroup::Perfect) => self.past_perfect(person, plural),
            (TenseTime::Past, TenseGroup::PerfectContinuous) => {
                self.past_perfect_continuous(person, plural)
            }

            (TenseTime::Future, TenseGroup::Simple) => self.future_simple(person, plural),
            (TenseTime::Future, TenseGroup::Continuous) => self.future_continuous(person, plural),
            (TenseTime::Future, TenseGroup::Perfect) => self.future_perfect(person, plural),
            (TenseTime::Future, TenseGroup::PerfectContinuous) => {
                self.future_perfect_continuous(person, plural)
            }
        }
    }

    fn first_form(&self) -> String;
    fn second_form(&self) -> String {
        self.first_form()
    }
    fn third_form(&self) -> String {
        let base = self.first_form();
        let last = base.chars().last().unwrap();
        if is_vowel(last) {
            return base + "d";
        }
        base + "ed"
    }
    fn continuous_form(&self) -> String {
        let base = self.first_form();
        let last = match base.chars().next_back() {
            Some(c) => c,
            None => panic!("empty string \"{base}\""),
        };

        if is_vowel(last) {
            let stem = &base[..base.len() - last.len_utf8()];
            return stem.to_string() + "ing";
        }

        base + "ing"
    }
    fn tperson_form(&self) -> String {
        self.first_form() + "s"
    }

    fn present_simple(&self, person: Person, plural: bool) -> String {
        match (person, plural) {
            (Person::Third, false) => self.tperson_form(),
            _ => self.first_form(),
        }
    }
    fn present_continuous(&self, person: Person, plural: bool) -> String {
        Be.present_simple(person, plural) + " " + &self.continuous_form()
    }
    fn present_perfect(&self, person: Person, plural: bool) -> String {
        Have.present_simple(person, plural) + " " + &self.third_form()
    }
    fn present_perfect_continuous(&self, person: Person, plural: bool) -> String {
        Be.present_perfect(person, plural) + " " + &self.continuous_form()
    }

    fn past_simple(&self, _person: Person, _plural: bool) -> String {
        self.second_form()
    }
    fn past_continuous(&self, person: Person, plural: bool) -> String {
        Be.past_simple(person, plural) + " " + &self.continuous_form()
    }
    fn past_perfect(&self, person: Person, plural: bool) -> String {
        Have.past_simple(person, plural) + " " + &self.third_form()
    }
    fn past_perfect_continuous(&self, person: Person, plural: bool) -> String {
        Be.past_perfect(person, plural) + " " + &self.continuous_form()
    }

    fn future_simple(&self, _person: Person, _plural: bool) -> String {
        String::from("will ") + &self.first_form()
    }
    fn future_continuous(&self, person: Person, plural: bool) -> String {
        Be.future_simple(person, plural) + " " + &self.continuous_form()
    }
    fn future_perfect(&self, person: Person, plural: bool) -> String {
        Have.future_simple(person, plural) + " " + &self.third_form()
    }
    fn future_perfect_continuous(&self, person: Person, plural: bool) -> String {
        Be.future_perfect(person, plural) + " " + &self.continuous_form()
    }
}
