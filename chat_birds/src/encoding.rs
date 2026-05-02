use crate::belief::SubjectBeliefs;
use crate::states::StateAsRepr;
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

fn encode_sentence(
    from: AgentId,
    to: AgentId,
    subject: &String,
    beliefs: &SubjectBeliefs,
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
    let mut verb_groups: HashMap<String, Vec<String>> = HashMap::new();

    for (_type_id, belief_set) in beliefs.map.iter() {
        for belief in belief_set {
            // Prefer richer representation when the concrete state exposes it
            // via `StateRepr`. This works for trait objects because the
            // registry in `states.rs` lets us locate the concrete implementation.
            if let Some(state_repr) = belief.state.as_state_repr() {
                let verb = state_repr.verb();
                let object = state_repr.object();

                let tense = Tense {
                    time: TenseTime::Present,
                    group: TenseGroup::Simple,
                };
                let conjugated = verb.get(person, plural, tense);

                verb_groups.entry(conjugated).or_default().push(object);
            } else {
                // Fallback: use type name-based object and default verb
                let state_ref: &dyn crate::State = belief.state.as_ref();
                let object = short_type_name(std::any::type_name_of_val(state_ref)).to_lowercase();
                let verb = crate::verb::Verb::be();
                let tense = Tense {
                    time: TenseTime::Present,
                    group: TenseGroup::Simple,
                };
                let conjugated = verb.get(person, plural, tense);
                verb_groups.entry(conjugated).or_default().push(object);
            }
        }
    }

    // ------------------------------------------------------------------------
    // 3. Format output: "<subject> <verb1> <objs>, <verb2> <objs>, ..."
    // ------------------------------------------------------------------------
    if verb_groups.is_empty() {
        return format!("{} .", subject_string);
    }

    let mut clauses = Vec::new();

    for (conjugated_verb, mut objects) in verb_groups {
        // Deduplicate and sort for deterministic output
        objects.sort();
        objects.dedup();

        let objects_str = format_list(objects);
        clauses.push(format!("{} {}", conjugated_verb, objects_str));
    }

    clauses.sort();

    format!("{} {}.", subject_string, clauses.join(", "))
}
