use crate::Clock;
use std::cmp::Ordering;

use crate::{
    verb::{Tense, TenseGroup},
    Temporal, TenseTime, Timestamp,
};

pub trait ToTense {
    fn to_tense(&self, story: Temporal, scope: Timestamp, clock: &Clock) -> Tense;
}

fn is_lt(c: Option<Ordering>) -> bool {
    matches!(c, Some(Ordering::Less))
}

fn is_le(c: Option<Ordering>) -> bool {
    matches!(c, Some(Ordering::Less | Ordering::Equal))
}

fn is_eq(c: Option<Ordering>) -> bool {
    matches!(c, Some(Ordering::Equal))
}

fn is_ge(c: Option<Ordering>) -> bool {
    matches!(c, Some(Ordering::Greater | Ordering::Equal))
}

fn match_present(t: &Temporal, story: &Temporal, scope: Timestamp, clock: &Clock) -> TenseGroup {
    let _ = story;
    match t {
        Temporal::Always => TenseGroup::Simple,
        Temporal::Tense(TenseTime::Past) => TenseGroup::Perfect,
        Temporal::Period { start, end } => {
            let start_now = clock.cmp_temporal_with_scope(start.as_ref(), &scope);
            let end_now = clock.cmp_temporal_with_scope(end.as_ref(), &scope);

            // started before now and still relevant through now
            if is_lt(start_now) && is_ge(end_now) {
                return TenseGroup::PerfectContinuous;
            }
            // ongoing around now
            if is_le(start_now) && is_ge(end_now) {
                return TenseGroup::Continuous;
            }
            // finished before now but still framed from present context
            if is_lt(end_now) {
                return TenseGroup::Perfect;
            }
            TenseGroup::Simple
        }
        Temporal::Timestamp(_) => {
            let cmp_now = clock.cmp_temporal_with_scope(t, &scope);
            if is_lt(cmp_now) {
                TenseGroup::Perfect
            } else {
                TenseGroup::Simple
            }
        }
        _ => TenseGroup::Simple,
    }
}

fn match_past(t: &Temporal, story: &Temporal, _scope: Timestamp, _clock: &Clock) -> TenseGroup {
    match t {
        Temporal::Period { start, end } => {
            let start_story = start.as_ref().partial_cmp(story);
            let end_story = end.as_ref().partial_cmp(story);

            // start < story <= end
            if is_lt(start_story) && is_ge(end_story) {
                return TenseGroup::PerfectContinuous;
            }
            // start <= story <= end
            if is_le(start_story) && is_ge(end_story) {
                return TenseGroup::Continuous;
            }
            // period finishes before story reference
            if is_lt(end_story) {
                if matches!(start.as_ref(), Temporal::Timestamp(_)) {
                    return TenseGroup::Simple;
                }
                return TenseGroup::Perfect;
            }
            TenseGroup::Simple
        }
        Temporal::Timestamp(_) => {
            let cmp_story = t.partial_cmp(story);
            if is_eq(cmp_story) {
                TenseGroup::Simple
            } else if is_lt(cmp_story) {
                TenseGroup::Perfect
            } else {
                TenseGroup::Simple
            }
        }
        Temporal::Tense(TenseTime::Past) => TenseGroup::Simple,
        _ => TenseGroup::Simple,
    }
}

fn match_future(t: &Temporal, story: &Temporal, _scope: Timestamp, _clock: &Clock) -> TenseGroup {
    match t {
        Temporal::Tense(TenseTime::Future) => TenseGroup::Simple,
        Temporal::Period { start, end } => {
            let start_story = start.as_ref().partial_cmp(story);
            let end_story = end.as_ref().partial_cmp(story);

            // start < story <= end
            if is_lt(start_story) && is_ge(end_story) {
                return TenseGroup::PerfectContinuous;
            }
            // story <= end with broad/unanchored start
            if is_ge(end_story) && matches!(start.as_ref(), Temporal::Tense(_) | Temporal::Always) {
                return TenseGroup::Continuous;
            }
            // finished before future story point
            if is_lt(end_story) {
                return TenseGroup::Perfect;
            }
            TenseGroup::Simple
        }
        Temporal::Timestamp(_) => {
            let cmp_story = t.partial_cmp(story);
            if is_eq(cmp_story) {
                TenseGroup::Simple
            } else if is_lt(cmp_story) {
                TenseGroup::Perfect
            } else {
                TenseGroup::Simple
            }
        }
        _ => TenseGroup::Simple,
    }
}

impl ToTense for Temporal {
    fn to_tense(&self, story: Temporal, scope: Timestamp, clock: &Clock) -> Tense {
        /*
        ## Present : time of story =~ current time
        Present simple :
        - Habits / repeated actions / unchanging situations : ?
        - General truth : Temporal::Always
        - Emotions / wishes : Temporal::Timestamp(=~ current time)
        Present continuous :
        - Ongoing temporary action : Temporal::Period(start <= current time <= end; start : Tense or Always)
        Present perfect :
        - Finished action, relevant to present, no specific time : Temporal::Tense(Past)
        Present perfect continuous :
        - Ongoing temporary action that has started in the past : Temporal::Period(start < current time <= end)

        ## Past : time of story < current time
        Past simple :
        - Finished action from a specific time :
            - Temporal::Period(start <= end < current time; start : Timestamp)
            - Temporal::Timestamp(= time of story)
        Past continuous :
        - Ongoing action at time of story : Temporal::Period(start <= time of story <= end; start : Tense or Always)
        Past perfect :
        - Finished action at time of story :
            - Temporal::Period(start <= end < time of story)
            - Temporal::Timestamp(< time of story)
        Past perfect continuous :
        - Ongoing action that had started before time of story : Temporal::Period(start < time of story <= end)

        ## Future: time of story > current time
        Future simple :
        - Action that will happen :
            - Temporal::TimeStamp(= time of story)
            - Temporal::Tense(Future)
        Future continuous :
        - Action that will be ongoing at time of story : Temporal::Period(time of story <= end; start : Tense or Always)
        Future perfect :
        - Finished action at time of story :
            - Temporal::Period(start <= end < time of story)
            - Temporal::Timestamp(< time of story)
        Future perfect continuous :
        - Ongoing action that had started before time of story : Temporal::Period(start < time of story <= end)
        */
        let cmp = clock.cmp_temporal_with_scope(&story, &scope);

        match cmp {
            Some(Ordering::Greater) => Tense {
                time: TenseTime::Future,
                group: match_future(&self, &story, scope, clock),
            },
            Some(Ordering::Equal) => Tense {
                time: TenseTime::Present,
                group: match_present(&self, &story, scope, clock),
            },
            Some(Ordering::Less) => Tense {
                time: TenseTime::Past,
                group: match_past(&self, &story, scope, clock),
            },
            None => {
                panic!("Impossible comparison between current time and time of story")
            }
        }
    }
}
