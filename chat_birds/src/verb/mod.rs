pub mod person;
pub use person::{InvalidPerson, Person};

use chat_birds_core::TenseTime;

const VOWELS: [char; 5] = ['a', 'e', 'i', 'o', 'u'];

fn is_vowel(c: char) -> bool {
    VOWELS.contains(&c)
}

fn is_consonant(c: char) -> bool {
    c.is_ascii_alphabetic() && !is_vowel(c)
}

fn should_double_final_consonant(base: &str) -> bool {
    let mut chars = base.chars().rev();
    let Some(last) = chars.next() else {
        return false;
    };
    let Some(middle) = chars.next() else {
        return false;
    };
    let Some(prev) = chars.next() else {
        return false;
    };

    is_consonant(prev) && is_vowel(middle) && is_consonant(last) && !matches!(last, 'w' | 'x' | 'y')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenseGroup {
    Simple,
    Continuous,
    Perfect,
    PerfectContinuous,
}

#[derive(Debug, Clone, Copy)]
pub struct Tense {
    pub(crate) time: TenseTime,
    pub(crate) group: TenseGroup,
}

/// A verb with its conjugation patterns.
///
/// Handles regular verbs, semi-irregular verbs (like "have"),
/// and fully suppletive verbs (like "be") in a single type.
#[derive(Debug, Clone)]
pub enum Verb {
    /// Regular verb following standard -ed/-ing rules.
    /// Example: "walk" → walked, walked, walking
    Regular { base: String },

    /// Semi-irregular verb: custom past/participle/3rd-person,
    /// but otherwise follows regular patterns.
    /// Example: "have" → had, had, has, having
    SemiIrregular {
        base: String,
        past: String,
        /// None = same as past
        past_participle: Option<String>,
        /// None = base + "ing"
        present_participle: Option<String>,
        /// None = base + "s"
        third_person: Option<String>,
    },

    /// Fully suppletive verb with custom conjugation functions.
    /// Example: "be" → am/is/are, was/were, been, being
    Suppletive {
        infinitive: String,
        /// Conjugation for present simple tense
        present_simple_fn: fn(Person, bool) -> String,
        /// Conjugation for past simple tense  
        past_simple_fn: fn(Person, bool) -> String,
        past_participle: String,
        present_participle: String,
    },
}

impl Verb {
    // ========================================================================
    // Constructors
    // ========================================================================

    /// Create a regular verb from its base form.
    /// Example: `Verb::regular("walk")`
    pub fn regular(base: impl Into<String>) -> Self {
        Self::Regular { base: base.into() }
    }

    /// Create the verb "be" with its suppletive conjugations.
    pub fn be() -> Self {
        Self::Suppletive {
            infinitive: "be".into(),
            present_simple_fn: |person, plural| match (person, plural) {
                (Person::First, false) => "am".into(),
                (Person::Second, false) | (_, true) => "are".into(),
                (Person::Third, false) => "is".into(),
            },
            past_simple_fn: |person, plural| match (person, plural) {
                (Person::First, false) | (Person::Third, false) => "was".into(),
                (Person::Second, false) | (_, true) => "were".into(),
            },
            past_participle: "been".into(),
            present_participle: "being".into(),
        }
    }

    /// Create the verb "have" with its irregular forms.
    pub fn have() -> Self {
        Self::SemiIrregular {
            base: "have".into(),
            past: "had".into(),
            past_participle: Some("had".into()),
            present_participle: None, // defaults to "having"
            third_person: Some("has".into()),
        }
    }

    // ========================================================================
    // Core conjugation entry point (replaces trait's default `get`)
    // ========================================================================

    pub fn get(&self, person: Person, plural: bool, tense: Tense) -> String {
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

    // ========================================================================
    // Base form accessors (replaces trait methods)
    // ========================================================================

    pub fn first_form(&self) -> String {
        match self {
            Verb::Regular { base } => base.clone(),
            Verb::SemiIrregular { base, .. } => base.clone(),
            Verb::Suppletive { infinitive, .. } => infinitive.clone(),
        }
    }

    pub fn second_form(&self) -> String {
        match self {
            Verb::Regular { base } => self.default_past(base),
            Verb::SemiIrregular { past, .. } => past.clone(),
            Verb::Suppletive { past_simple_fn, .. } => {
                // For suppletive verbs, past simple is the same for all persons in 2nd form context
                past_simple_fn(Person::Third, false)
            }
        }
    }

    pub fn third_form(&self) -> String {
        match self {
            Verb::Regular { base } => self.default_past_participle(base),
            Verb::SemiIrregular {
                past_participle,
                past,
                ..
            } => past_participle.clone().unwrap_or_else(|| past.clone()),
            Verb::Suppletive {
                past_participle, ..
            } => past_participle.clone(),
        }
    }

    pub fn continuous_form(&self) -> String {
        match self {
            Verb::Regular { base } => self.default_continuous(base),
            Verb::SemiIrregular {
                present_participle,
                base,
                ..
            } => present_participle
                .clone()
                .unwrap_or_else(|| self.default_continuous(base)),
            Verb::Suppletive {
                present_participle, ..
            } => present_participle.clone(),
        }
    }

    pub fn tperson_form(&self) -> String {
        match self {
            Verb::Regular { base } => self.default_third_person(base),
            Verb::SemiIrregular {
                third_person, base, ..
            } => third_person
                .clone()
                .unwrap_or_else(|| self.default_third_person(base)),
            Verb::Suppletive {
                present_simple_fn, ..
            } => present_simple_fn(Person::Third, false),
        }
    }

    // ========================================================================
    // Default conjugation logic for regular verbs
    // ========================================================================

    fn default_past(&self, base: &str) -> String {
        if base.is_empty() {
            return String::new();
        }
        let last = base.chars().last().unwrap();
        if base.ends_with('e') {
            format!("{}d", base)
        } else if is_vowel(last) {
            format!("{}d", base)
        } else {
            format!("{}ed", base)
        }
    }

    fn default_past_participle(&self, base: &str) -> String {
        self.default_past(base)
    }

    fn default_continuous(&self, base: &str) -> String {
        if base.is_empty() {
            return "ing".into();
        }
        let last = base.chars().last().unwrap();

        // Simple -ing rule: drop final 'e', add "ing"
        if base.ends_with('e') && base.len() > 1 {
            return format!("{}ing", &base[..base.len() - 1]);
        }

        if should_double_final_consonant(base) {
            return format!("{}{}ing", base, last);
        }

        // Simple vowel+consonant doubling could go here if needed
        format!("{}ing", base)
    }

    fn default_third_person(&self, base: &str) -> String {
        if base.is_empty() {
            return "s".into();
        }
        if base.ends_with('s')
            || base.ends_with("sh")
            || base.ends_with("ch")
            || base.ends_with('x')
            || base.ends_with('z')
            || base.ends_with('o')
        {
            format!("{}es", base)
        } else if base.ends_with('y') && !base[..base.len() - 1].ends_with(is_vowel) {
            format!("{}ies", &base[..base.len() - 1])
        } else {
            format!("{}s", base)
        }
    }

    // ========================================================================
    // Tense implementations (delegates to base form methods)
    // ========================================================================

    fn present_simple(&self, person: Person, plural: bool) -> String {
        match self {
            Verb::Suppletive {
                present_simple_fn, ..
            } => present_simple_fn(person, plural),
            _ => match (person, plural) {
                (Person::Third, false) => self.tperson_form(),
                _ => self.first_form(),
            },
        }
    }

    fn present_continuous(&self, person: Person, plural: bool) -> String {
        // "be" is special: it doesn't use itself as auxiliary in continuous
        let aux = if matches!(self, Verb::Suppletive { infinitive, .. } if infinitive == "be") {
            Self::be().present_simple(person, plural)
        } else {
            Self::be().present_simple(person, plural)
        };
        format!("{} {}", aux, self.continuous_form())
    }

    fn present_perfect(&self, person: Person, plural: bool) -> String {
        let aux = Self::have().present_simple(person, plural);
        format!("{} {}", aux, self.third_form())
    }

    fn present_perfect_continuous(&self, person: Person, plural: bool) -> String {
        let aux = Self::be().present_perfect(person, plural);
        format!("{} {}", aux, self.continuous_form())
    }

    fn past_simple(&self, person: Person, plural: bool) -> String {
        match self {
            Verb::Suppletive { past_simple_fn, .. } => past_simple_fn(person, plural),
            _ => self.second_form(),
        }
    }

    fn past_continuous(&self, person: Person, plural: bool) -> String {
        let aux = Self::be().past_simple(person, plural);
        format!("{} {}", aux, self.continuous_form())
    }

    fn past_perfect(&self, person: Person, plural: bool) -> String {
        let aux = Self::have().past_simple(person, plural);
        format!("{} {}", aux, self.third_form())
    }

    fn past_perfect_continuous(&self, person: Person, plural: bool) -> String {
        let aux = Self::be().past_perfect(person, plural);
        format!("{} {}", aux, self.continuous_form())
    }

    fn future_simple(&self, _person: Person, _plural: bool) -> String {
        format!("will {}", self.first_form())
    }

    fn future_continuous(&self, person: Person, plural: bool) -> String {
        let aux = Self::be().future_simple(person, plural);
        format!("{} {}", aux, self.continuous_form())
    }

    fn future_perfect(&self, person: Person, plural: bool) -> String {
        let aux = Self::have().future_simple(person, plural);
        format!("{} {}", aux, self.third_form())
    }

    fn future_perfect_continuous(&self, person: Person, plural: bool) -> String {
        let aux = Self::be().future_perfect(person, plural);
        format!("{} {}", aux, self.continuous_form())
    }
}

// ============================================================================
// Re-export Be and Have as convenient constants/functions for backward compat
// ============================================================================

/// The verb "be" – use `Verb::be()` instead.
#[deprecated(since = "2.0.0", note = "Use `Verb::be()` instead")]
pub struct Be;
#[allow(deprecated)]
impl Be {
    #[deprecated(since = "2.0.0", note = "Use `Verb::be()` instead")]
    pub fn instance() -> Verb {
        Verb::be()
    }
}

/// The verb "have" – use `Verb::have()` instead.
#[deprecated(since = "2.0.0", note = "Use `Verb::have()` instead")]
pub struct Have;
#[allow(deprecated)]
impl Have {
    #[deprecated(since = "2.0.0", note = "Use `Verb::have()` instead")]
    pub fn instance() -> Verb {
        Verb::have()
    }
}
