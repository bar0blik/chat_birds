use chat_birds_core::{Clock, Message, MessageCodec, Temporal, Timestamp};

use crate::belief::SubjectBeliefs;
use crate::states::{ObjectFragment, StateAsRepr};
use crate::time::ToTense;
use crate::verb::{Person, Tense, TenseGroup};
use crate::BeliefKey;
use crate::{AgentId, TenseTime}; // bring the extension trait into scope

use std::collections::HashMap; // Adjust path to your StateRepr trait

/// Helper: format a list with commas and "and" before the last item
fn format_list(items: Vec<String>) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let mut s = items[..items.len() - 1].join(", ");
            s.push_str(" and ");
            s.push_str(&items[items.len() - 1]);
            s
        }
    }
}

fn short_type_name(full: &str) -> &str {
    full.rsplit("::").next().unwrap_or(full)
}

fn orchestrate_fragments(fragments: &mut [ObjectFragment]) -> String {
    // Partition by type and sort adjectives by category
    let mut adjectives: Vec<(crate::states::AdjectiveCategory, &str)> = Vec::new();
    let mut nouns = Vec::new();

    for frag in fragments.iter() {
        match frag {
            ObjectFragment::Adjective { lemma, category } => {
                adjectives.push((*category, lemma.as_str()))
            }
            ObjectFragment::Noun { lemma, .. } => nouns.push(lemma.as_str()),
            // Skip or handle other variants later
            _ => {}
        }
    }

    // Sort adjectives by category (using natural order from AdjectiveCategory)
    adjectives.sort_by_key(|(cat, _)| *cat);

    // No nouns? Just join what we have
    if nouns.is_empty() {
        return if adjectives.is_empty() {
            String::new()
        } else {
            adjectives.iter().map(|(_, lemma)| *lemma).collect::<Vec<_>>().join(" ")
        };
    }

    // Take first noun as head, prepend adjectives
    let head_noun = nouns[0];
    let adj_str = adjectives
        .iter()
        .map(|(_, lemma)| *lemma)
        .collect::<Vec<_>>()
        .join(" ");
    let combined = if adj_str.is_empty() {
        head_noun.to_string()
    } else {
        format!("{} {}", adj_str, head_noun)
    };

    // Add simple indefinite article based on the first word of the combined phrase
    let first_word = combined.split_whitespace().next().unwrap_or(head_noun);
    let article = if first_word
        .chars()
        .next()
        .map(|c| "aeiouAEIOU".contains(c))
        .unwrap_or(false)
    {
        "an"
    } else {
        "a"
    };

    let mut result = format!("{} {}", article, combined);

    // Append extra nouns with "and" (fallback for multiple heads)
    for extra_noun in nouns.iter().skip(1) {
        let extra_article = if extra_noun
            .chars()
            .next()
            .map(|c| "aeiouAEIOU".contains(c))
            .unwrap_or(false)
        {
            "an"
        } else {
            "a"
        };
        result = format!("{} and {} {}", result, extra_article, extra_noun);
    }

    result
}

fn encode_sentence(
    from: AgentId,
    to: AgentId,
    subject: &String,
    beliefs: &SubjectBeliefs,
    clock: &Clock,
) -> String {
    // ------------------------------------------------------------------------
    // 1. Determine grammatical person for conjugation
    // ------------------------------------------------------------------------
    let from_key = from.to_key();
    let to_key = to.to_key();

    let (subject_string, person): (String, Person) = if subject.as_str() == from_key.as_ref() {
        (String::from("I"), Person::First)
    } else if subject.as_str() == to_key.as_ref() {
        (String::from("You"), Person::Second)
    } else {
        (subject.clone(), Person::Third)
    };

    let plural = beliefs.plural;

    // ------------------------------------------------------------------------
    // 2. Group beliefs by conjugated verb using StateRepr trait
    // ------------------------------------------------------------------------
    let mut verb_groups: HashMap<String, Vec<ObjectFragment>> = HashMap::new();

    for (_type_id, belief_set) in beliefs.map.iter() {
        for belief in belief_set {
            // Prefer richer representation when the concrete state exposes it
            // via `StateRepr`. This works for trait objects because the
            // registry in `states.rs` lets us locate the concrete implementation.
            if let Some(state_repr) = belief.state.as_state_repr() {
                let verb = state_repr.verb();
                let object_opt = state_repr.object();

                let tense = Tense {
                    time: TenseTime::Present,
                    group: TenseGroup::Simple,
                };
                let conjugated = verb.get(person, plural, tense);

                let entry = verb_groups.entry(conjugated).or_default();
                if let Some(obj_frag) = object_opt {
                    entry.push(obj_frag);
                } else {
                    // Intentionally ignore None: produce no object text for this verb
                    // ensure the verb key exists so we can emit a bare verb clause later
                    let _ = entry;
                }
            } else {
                dbg!(
                    "
                =====================================================\n
                           Failed to downcast to StateRepr\n
                =====================================================
                "
                );
                // Fallback: use type name-based object and default verb
                let state_ref: &dyn crate::State = belief.state.as_ref();
                let object = ObjectFragment::noun(
                    short_type_name(std::any::type_name_of_val(state_ref)).to_lowercase(),
                );
                let verb = crate::verb::Verb::be();

                // TODO: get scope and time of story to compute tense
                let story = Temporal::Timestamp(clock.now());
                let mut scope = Timestamp::empty();
                scope.set_second(Some(10));

                let tense = belief.temporal.to_tense(story, scope, clock);
                let conjugated = verb.get(person, plural, tense);
                verb_groups.entry(conjugated).or_default().push(object);
            }
        }
    }

    // ------------------------------------------------------------------------
    // 3. Format output: "<subject> <verb1> <objs>, <subject> <verb2> <objs>"
    // ------------------------------------------------------------------------
    if verb_groups.is_empty() {
        return format!("{}.", subject_string);
    }

    let mut clauses = Vec::new();

    for (conjugated_verb, mut fragments) in verb_groups {
        let objects_str = orchestrate_fragments(&mut fragments);
        let clause = if objects_str.is_empty() {
            format!("{} {}", subject_string, conjugated_verb)
        } else {
            format!("{} {} {}", subject_string, conjugated_verb, objects_str)
        };
        clauses.push(clause);
    }

    clauses.sort();

    format!("{}.", format_list(clauses))
}

pub struct DefaultCodec;

impl MessageCodec for DefaultCodec {
    fn decode(&self, s: &str, from: AgentId, to: AgentId) -> Option<Message> {
        None
    }
    fn encode(&self, msg: &Message, clock: &Clock) -> String {
        let mut s = String::new();
        for (k, v) in &msg.payload {
            s += &encode_sentence(msg.from, msg.to, k, v, clock);
        }
        s
    }
}
