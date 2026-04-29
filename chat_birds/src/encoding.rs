use crate::states::StateRepr;
use crate::verb::{Person, Verb};
use chat_birds_core::belief::SubjectBeliefs;
use chat_birds_core::{AgentId, BeliefKey, Message, MessageCodec};
use std::collections::BTreeMap;
struct DefaultCodec;

pub fn encode_message(msg: &Message) -> String {
    DefaultCodec.encode(msg)
}

fn encode_sentence(
    from: AgentId,
    to: AgentId,
    subject: &String,
    beliefs: &SubjectBeliefs,
) -> String {
    // Find person
    let from_key = from.to_key();
    let to_key = to.to_key();
    let (subject_string, person): (String, Person) = if subject.as_str() == from_key.as_ref() {
        (String::from("I"), Person::First)
    } else if subject.as_str() == to_key.as_ref() {
        (String::from("You"), Person::Second)
    } else {
        (subject.clone(), Person::Third)
    };

    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (_tid, entries) in beliefs.map.iter() {
        if let Some(best) = entries.iter().max_by_key(|e| e.certainty) {
            let state = best
                .state
                .as_ref()
                .as_any()
                .downcast_ref::<dyn StateRepr>()
                .unwrap();
            let verb = state.verb().present_simple(person, false);
            let object = state.object();
            grouped
                .entry(format!("{} {}", verb, object))
                .or_default()
                .push(String::new());
        }
    }

    if grouped.is_empty() {
        let aux = match person {
            Person::Third => "has",
            _ => "have",
        };
        return format!("{} {} no beliefs.", subject_string, aux);
    }

    let mut clauses = Vec::new();
    for (phrase, _) in grouped {
        clauses.push(phrase);
    }

    if clauses.is_empty() {
        let aux = match person {
            Person::Third => "has",
            _ => "have",
        };
        return format!("{} {} no beliefs.", subject_string, aux);
    }

    let phrases = match clauses.len() {
        1 => clauses[0].clone(),
        2 => format!("{} and {}", clauses[0], clauses[1]),
        _ => {
            let last = clauses.pop().unwrap_or_default();
            format!("{}, and {}", clauses.join(", "), last)
        }
    };

    format!("{} {}.", subject_string, phrases)
}

impl MessageCodec for DefaultCodec {
    fn encode(&self, msg: &Message) -> String {
        let mut subjects: Vec<&String> = msg.payload.0.keys().collect();
        subjects.sort_unstable();

        subjects
            .into_iter()
            .filter_map(|subject| {
                msg.payload
                    .get(subject)
                    .map(|beliefs| encode_sentence(msg.from, msg.to, subject, beliefs))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn decode(&self, _s: &str, _from: AgentId, _to: AgentId) -> Option<Message> {
        None
    }
}
