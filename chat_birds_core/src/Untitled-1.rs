/// Match an exact certainty value or a certainty range.
#[derive(Clone, Debug)]
pub enum U8Query {
    Exact(u8),
    Range { min: u8, max: u8 },
}

/// Match certainty on a belief entry.
pub type CertaintyQuery = U8Query;

/// Match the numeric level inside `Probability::Level`.
pub type ProbabilityLevelQuery = U8Query;

/// Match the condition string inside `Probability::Condition`.
#[derive(Clone, Debug)]
pub enum ConditionQuery {
    Any,
    Exact(String),
}

/// Match a belief's probability field.
#[derive(Clone, Debug)]
pub enum ProbabilityQuery {
    Any,
    Level(ProbabilityLevelQuery),
    Condition(ConditionQuery),
    Always,
    Never,
}

/// Match a belief's source field.
#[derive(Clone, Debug)]
pub enum SourceQuery {
    Any,
    Myself,
    Inferred,
    Agent(Option<AgentId>),
}

/// Compare a belief's time against a current timestamp.
#[derive(Clone, Debug)]
pub enum TimeQuery {
    Any,
    Exact(Timestamp),
    RelativeToNow {
        now: Timestamp,
        relation: TimeRelation,
    },
    Tense(TenseTime),
    Always,
}

/// Relative time relation for comparing a belief against the current time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeRelation {
    Past,
    Present,
    Future,
}

/// Query parameters to search a belief map.
#[derive(Clone, Debug, Default)]
pub struct QueryParam {
    pub state_id: Option<TypeId>,
    pub certainty: Option<CertaintyQuery>,
    pub probability: Option<ProbabilityQuery>,
    pub source: Option<SourceQuery>,
    pub time: Option<TimeQuery>,
}

impl QueryParam {
    /// Create a query restricted to a concrete state type.
    pub fn for_state<S: State + 'static>() -> Self {
        Self {
            state_id: Some(TypeId::of::<S>()),
            ..Self::default()
        }
    }
}

/// Return the highest-certainty belief entry that matches the query.
pub fn query(&self, param: QueryParam) -> Option<&BeliefEntry> {
    self.0
        .iter()
        .filter(|(state_id, entries)| {
            param
                .state_id
                .map(|wanted| wanted == **state_id)
                .unwrap_or(true)
                && !entries.is_empty()
        })
        .flat_map(|(_, entries)| entries.iter())
        .filter(|entry| entry_matches_query(entry, &param))
        .max_by(|a, b| a.certainty.cmp(&b.certainty))
}

fn entry_matches_query(entry: &BeliefEntry, param: &QueryParam) -> bool {
    matches_state_id(entry, param.state_id)
        && matches_certainty(entry.certainty, param.certainty.as_ref())
        && matches_probability(&entry.probability, param.probability.as_ref())
        && matches_source(&entry.source, param.source.as_ref())
        && matches_time(&entry.temporal, param.time.as_ref())
}

fn matches_state_id(entry: &BeliefEntry, state_id: Option<TypeId>) -> bool {
    state_id
        .map(|wanted| entry.state.as_any().type_id() == wanted)
        .unwrap_or(true)
}

fn matches_certainty(certainty: u8, query: Option<&CertaintyQuery>) -> bool {
    match query {
        None => true,
        Some(CertaintyQuery::Exact(value)) => certainty == *value,
        Some(CertaintyQuery::Range { min, max }) => (certainty >= *min) && (certainty <= *max),
    }
}

fn matches_probability(probability: &Probability, query: Option<&ProbabilityQuery>) -> bool {
    match query {
        None | Some(ProbabilityQuery::Any) => true,
        Some(ProbabilityQuery::Level(level_query)) => match probability {
            Probability::Level(value) => matches_u8_query(*value, level_query),
            _ => false,
        },
        Some(ProbabilityQuery::Condition(condition_query)) => {
            match (probability, condition_query) {
                (Probability::Condition(_), ConditionQuery::Any) => true,
                (Probability::Condition(actual), ConditionQuery::Exact(expected)) => {
                    actual == expected
                }
                _ => false,
            }
        }
        Some(ProbabilityQuery::Always) => matches!(probability, Probability::Always),
        Some(ProbabilityQuery::Never) => matches!(probability, Probability::Never),
    }
}

fn matches_source(source: &BeliefSource, query: Option<&SourceQuery>) -> bool {
    match query {
        None | Some(SourceQuery::Any) => true,
        Some(SourceQuery::Myself) => matches!(source, BeliefSource::Myself),
        Some(SourceQuery::Inferred) => matches!(source, BeliefSource::Inferred),
        Some(SourceQuery::Agent(None)) => matches!(source, BeliefSource::Agent(_)),
        Some(SourceQuery::Agent(Some(expected))) => {
            matches!(source, BeliefSource::Agent(actual) if actual == expected)
        }
    }
}

fn matches_time(temporal: &Temporal, query: Option<&TimeQuery>) -> bool {
    match query {
        None | Some(TimeQuery::Any) => true,
        Some(TimeQuery::Exact(expected)) => {
            matches!(temporal, Temporal::Timestamp(actual) if actual.cmp(expected) == Ordering::Equal)
        }
        Some(TimeQuery::RelativeToNow { now, relation }) => match temporal {
            Temporal::Timestamp(actual) => timestamp_relation(actual, now) == *relation,
            Temporal::Period { start, end } => {
                period_relation(start, end, now) == Some(relation.clone())
            }
            Temporal::Tense(tense) => tense_relation(tense) == Some(relation.clone()),
            Temporal::Always => *relation == TimeRelation::Present,
        },
        Some(TimeQuery::Tense(expected)) => matches!(
            (temporal, expected),
            (Temporal::Tense(TenseTime::Past), TenseTime::Past)
                | (Temporal::Tense(TenseTime::Present), TenseTime::Present)
                | (Temporal::Tense(TenseTime::Future), TenseTime::Future)
        ),
        Some(TimeQuery::Always) => matches!(temporal, Temporal::Always),
    }
}

fn matches_u8_query(value: u8, query: &U8Query) -> bool {
    match query {
        U8Query::Exact(expected) => value == *expected,
        U8Query::Range { min, max } => (value >= *min) && (value <= *max),
    }
}

fn timestamp_relation(actual: &Timestamp, now: &Timestamp) -> TimeRelation {
    match actual.cmp(now) {
        Ordering::Less => TimeRelation::Past,
        Ordering::Equal => TimeRelation::Present,
        Ordering::Greater => TimeRelation::Future,
    }
}

fn period_relation(start: &Timestamp, end: &Timestamp, now: &Timestamp) -> Option<TimeRelation> {
    if end.cmp(now) == Ordering::Less {
        Some(TimeRelation::Past)
    } else if start.cmp(now) == Ordering::Greater {
        Some(TimeRelation::Future)
    } else if start.cmp(now) != Ordering::Greater && end.cmp(now) != Ordering::Less {
        Some(TimeRelation::Present)
    } else {
        None
    }
}

fn tense_relation(tense: &TenseTime) -> Option<TimeRelation> {
    match tense {
        TenseTime::Past => Some(TimeRelation::Past),
        TenseTime::Present => Some(TimeRelation::Present),
        TenseTime::Future => Some(TimeRelation::Future),
    }
}

/// Compare two timestamps using their visible fields.
pub fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    (
        self.get_year(),
        self.get_month(),
        self.get_day(),
        self.get_hour(),
        self.get_minute(),
        self.get_second(),
    )
        .cmp(&(
            other.get_year(),
            other.get_month(),
            other.get_day(),
            other.get_hour(),
            other.get_minute(),
            other.get_second(),
        ))
}
