#![allow(dead_code)]

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
//  CHAT_BIRDS CORE
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AgentId(pub u16);

// ── State ─────────────────────────────────────────────────────────────────────

pub trait State: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn clone_box(&self) -> Box<dyn State>;
}

#[macro_export]
macro_rules! impl_state {
    ($t:ty) => {
        impl State for $t {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }
            fn clone_box(&self) -> Box<dyn State> {
                Box::new(self.clone())
            }
        }
    };
}

// ── StateMap (own, always-certain states) ─────────────────────────────────────

pub struct StateMap(pub HashMap<TypeId, Box<dyn State>>);

impl StateMap {
    pub fn new() -> Self {
        StateMap(HashMap::new())
    }

    pub fn insert<S: State + 'static>(&mut self, s: S) {
        self.0.insert(TypeId::of::<S>(), Box::new(s));
    }

    pub fn get<S: State + 'static>(&self) -> Option<&S> {
        self.0
            .get(&TypeId::of::<S>())
            .and_then(|b| b.as_any().downcast_ref::<S>())
    }

    pub fn get_mut<S: State + 'static>(&mut self) -> Option<&mut S> {
        self.0
            .get_mut(&TypeId::of::<S>())
            .and_then(|b| b.as_any_mut().downcast_mut::<S>())
    }

    pub fn remove<S: State + 'static>(&mut self) -> Option<Box<dyn State>> {
        self.0.remove(&TypeId::of::<S>())
    }

    pub fn remove_as<S: State + 'static>(&mut self) -> Option<S> {
        self.0
            .remove(&TypeId::of::<S>())
            .and_then(|b| b.into_any().downcast::<S>().ok())
            .map(|b| *b)
    }
}

// ── Probability ───────────────────────────────────────────────────────────────
//
// Condition(key): holds if the belief at `key` is present and certain enough.
// Gets changed to "always" if the condition is found to be true
// TODO: maybe change condition to hold a &BeliefEntry

#[derive(Clone, Debug)]
pub enum Probability {
    Level(u8),         // 0 = impossible, 255 = certain
    Condition(String), // belief-store subject key that must hold
    Always,
    Never,
}

// ── Temporal ──────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum Tense {
    Past,
    Present,
    Future,
}

// == Year-optimized packing ====================================================
// A fully specified timestamp uses:
// 12 (months) * 31 (days) * 24 (hours) * 60 (minutes) * 60 (seconds)
// = 32_140_800 states, which fits in 25 bits.
//
// That leaves 39 bits for the year:
// -274_877_906_945 <= year <= 274_877_906_944
// (~549 billion representable years).

// == Padded packing ============================================================
// Field widths by range:
// month  : 4 bits  (0..11)
// day    : 5 bits  (0..30)
// hour   : 5 bits  (0..23)
// minute : 6 bits  (0..59)
// second : 6 bits  (0..59)
// Total  : 26 bits.
//
// That leaves 38 bits for the year:
// -137_438_953_473 <= year <= 137_438_953_472
// (~274 billion representable years).

// The padded layout is simpler to encode/decode and still provides a very large year range.
// In practice, this is far beyond typical world-simulation needs.

// Timestamp precision is represented by a 6-bit contiguous field mask.
// Invalid sparse masks (for example year + day without month) are not allowed.
/* 000000 000001 000011 000111 001111 011111 111111
         000010 000110 001110 011110 111110
         000100 001100 011100 111100
         001000 011000 111000
         010000 110000
         100000
*/

// There are 22 valid masks, so they can be encoded as a 5-bit shape index.
// We store that shape index in the top 5 bits of the 64-bit timestamp value.
// This keeps enough remaining payload bits for both sub-year fields and a large year range.
#[derive(Clone)]
pub struct Timestamp(u64);

impl Timestamp {
    const YEAR_BIT: u8 = 0b000001;
    const MONTH_BIT: u8 = 0b000010;
    const DAY_BIT: u8 = 0b000100;
    const HOUR_BIT: u8 = 0b001000;
    const MINUTE_BIT: u8 = 0b010000;
    const SECOND_BIT: u8 = 0b100000;

    const SUBYEAR_BITS: u64 = 25;
    const SUBYEAR_MASK: u64 = (1u64 << Self::SUBYEAR_BITS) - 1;
    const YEAR_BITS: u64 = 34;
    const YEAR_MASK: u64 = (1u64 << Self::YEAR_BITS) - 1;
    const PAYLOAD_MASK: u64 = (1u64 << 59) - 1;

    pub fn empty() -> Self {
        Timestamp(0)
    }

    fn subyear(&self) -> u32 {
        (self.0 & Self::SUBYEAR_MASK) as u32
    }

    fn set_subyear(&mut self, sub: u32) {
        self.0 = (self.0 & !Self::SUBYEAR_MASK) | u64::from(sub);
    }

    fn year_raw(&self) -> u64 {
        ((self.0 >> Self::SUBYEAR_BITS) & Self::YEAR_MASK) as u64
    }

    fn set_year_raw(&mut self, year: u64) {
        let year_bits = (year & Self::YEAR_MASK) << Self::SUBYEAR_BITS;
        self.0 = (self.0 & !(Self::YEAR_MASK << Self::SUBYEAR_BITS)) | year_bits;
    }

    fn decode_subyear(sub: u32) -> (u8, u8, u8, u8, u8) {
        let mut v = sub;
        let second = (v % 60) as u8;
        v /= 60;
        let minute = (v % 60) as u8;
        v /= 60;
        let hour = (v % 24) as u8;
        v /= 24;
        let day = (v % 31) as u8 + 1;
        v /= 31;
        let month = (v % 12) as u8 + 1;
        (month, day, hour, minute, second)
    }

    fn encode_subyear(month: u8, day: u8, hour: u8, minute: u8, second: u8) -> u32 {
        ((((u32::from(month - 1) * 31 + u32::from(day - 1)) * 24 + u32::from(hour)) * 60
            + u32::from(minute))
            * 60)
            + u32::from(second)
    }

    pub fn get_mask(&self) -> u8 {
        let shape = (self.0 >> 59) as u8;
        match shape {
            0 => 0b000000,
            1 => 0b000001,
            2 => 0b000011,
            3 => 0b000111,
            4 => 0b001111,
            5 => 0b011111,
            6 => 0b111111,
            7 => 0b000010,
            8 => 0b000110,
            9 => 0b001110,
            10 => 0b011110,
            11 => 0b111110,
            12 => 0b000100,
            13 => 0b001100,
            14 => 0b011100,
            15 => 0b111100,
            16 => 0b001000,
            17 => 0b011000,
            18 => 0b111000,
            19 => 0b010000,
            20 => 0b110000,
            21 => 0b100000,
            _ => {
                debug_assert!(false);
                0
            }
        }
    }

    pub fn set_mask(&mut self, mask: u8) -> bool {
        let shape = match mask {
            0b000000 => 0,
            0b000001 => 1,
            0b000011 => 2,
            0b000111 => 3,
            0b001111 => 4,
            0b011111 => 5,
            0b111111 => 6,
            0b000010 => 7,
            0b000110 => 8,
            0b001110 => 9,
            0b011110 => 10,
            0b111110 => 11,
            0b000100 => 12,
            0b001100 => 13,
            0b011100 => 14,
            0b111100 => 15,
            0b001000 => 16,
            0b011000 => 17,
            0b111000 => 18,
            0b010000 => 19,
            0b110000 => 20,
            0b100000 => 21,
            _ => return false,
        };

        self.0 = (self.0 & ((1u64 << 59) - 1)) | ((shape as u64) << 59);
        true
    }

    pub fn get_year(&self) -> Option<u64> {
        if self.get_mask() & Self::YEAR_BIT == 0 {
            None
        } else {
            Some(self.year_raw())
        }
    }

    pub fn set_year(&mut self, year: Option<u64>) -> bool {
        let mut mask = self.get_mask();
        match year {
            Some(y) if y <= Self::YEAR_MASK => mask |= Self::YEAR_BIT,
            Some(_) => return false,
            None => {
                mask &= !Self::YEAR_BIT;
            }
        }

        // Validate mask before touching any data.
        let mut probe = self.clone();
        if !probe.set_mask(mask) {
            return false;
        }

        // Mask is valid — commit both writes atomically.
        if let Some(y) = year {
            self.set_year_raw(y);
        }
        self.set_mask(mask);
        true
    }

    pub fn get_month(&self) -> Option<u8> {
        if self.get_mask() & Self::MONTH_BIT == 0 {
            None
        } else {
            Some(Self::decode_subyear(self.subyear()).0)
        }
    }

    pub fn set_month(&mut self, month: Option<u8>) -> bool {
        let mut mask = self.get_mask();
        let mut pending_subyear: Option<u32> = None;
        match month {
            Some(m) if (1..=12).contains(&m) => {
                let (_, day, hour, minute, second) = Self::decode_subyear(self.subyear());
                pending_subyear = Some(Self::encode_subyear(m, day, hour, minute, second));
                mask |= Self::MONTH_BIT;
            }
            Some(_) => return false,
            None => mask &= !Self::MONTH_BIT,
        }

        // Validate mask before touching any data.
        let mut probe = self.clone();
        if !probe.set_mask(mask) {
            return false;
        }

        // Mask is valid — commit both writes atomically.
        if let Some(sub) = pending_subyear {
            self.set_subyear(sub);
        }
        self.set_mask(mask);
        true
    }

    pub fn get_day(&self) -> Option<u8> {
        if self.get_mask() & Self::DAY_BIT == 0 {
            None
        } else {
            Some(Self::decode_subyear(self.subyear()).1)
        }
    }

    pub fn set_day(&mut self, day: Option<u8>) -> bool {
        let mut mask = self.get_mask();
        let mut pending_subyear: Option<u32> = None;
        match day {
            Some(d) if (1..=31).contains(&d) => {
                let (month, _, hour, minute, second) = Self::decode_subyear(self.subyear());
                pending_subyear = Some(Self::encode_subyear(month, d, hour, minute, second));
                mask |= Self::DAY_BIT;
            }
            Some(_) => return false,
            None => mask &= !Self::DAY_BIT,
        }

        // Validate mask before touching any data.
        let mut probe = self.clone();
        if !probe.set_mask(mask) {
            return false;
        }

        // Mask is valid — commit both writes atomically.
        if let Some(sub) = pending_subyear {
            self.set_subyear(sub);
        }
        self.set_mask(mask);
        true
    }

    pub fn get_hour(&self) -> Option<u8> {
        if self.get_mask() & Self::HOUR_BIT == 0 {
            None
        } else {
            Some(Self::decode_subyear(self.subyear()).2)
        }
    }

    pub fn set_hour(&mut self, hour: Option<u8>) -> bool {
        let mut mask = self.get_mask();
        let mut pending_subyear: Option<u32> = None;
        match hour {
            Some(h) if h <= 23 => {
                let (month, day, _, minute, second) = Self::decode_subyear(self.subyear());
                pending_subyear = Some(Self::encode_subyear(month, day, h, minute, second));
                mask |= Self::HOUR_BIT;
            }
            Some(_) => return false,
            None => mask &= !Self::HOUR_BIT,
        }

        // Validate mask before touching any data.
        let mut probe = self.clone();
        if !probe.set_mask(mask) {
            return false;
        }

        // Mask is valid — commit both writes atomically.
        if let Some(sub) = pending_subyear {
            self.set_subyear(sub);
        }
        self.set_mask(mask);
        true
    }

    pub fn get_minute(&self) -> Option<u8> {
        if self.get_mask() & Self::MINUTE_BIT == 0 {
            None
        } else {
            Some(Self::decode_subyear(self.subyear()).3)
        }
    }

    pub fn set_minute(&mut self, minute: Option<u8>) -> bool {
        let mut mask = self.get_mask();
        let mut pending_subyear: Option<u32> = None;
        match minute {
            Some(m) if m <= 59 => {
                let (month, day, hour, _, second) = Self::decode_subyear(self.subyear());
                pending_subyear = Some(Self::encode_subyear(month, day, hour, m, second));
                mask |= Self::MINUTE_BIT;
            }
            Some(_) => return false,
            None => mask &= !Self::MINUTE_BIT,
        }

        // Validate mask before touching any data.
        let mut probe = self.clone();
        if !probe.set_mask(mask) {
            return false;
        }

        // Mask is valid — commit both writes atomically.
        if let Some(sub) = pending_subyear {
            self.set_subyear(sub);
        }
        self.set_mask(mask);
        true
    }

    pub fn get_second(&self) -> Option<u8> {
        if self.get_mask() & Self::SECOND_BIT == 0 {
            None
        } else {
            Some(Self::decode_subyear(self.subyear()).4)
        }
    }

    pub fn set_second(&mut self, second: Option<u8>) -> bool {
        let mut mask = self.get_mask();
        let mut pending_subyear: Option<u32> = None;
        match second {
            Some(s) if s <= 59 => {
                let (month, day, hour, minute, _) = Self::decode_subyear(self.subyear());
                pending_subyear = Some(Self::encode_subyear(month, day, hour, minute, s));
                mask |= Self::SECOND_BIT;
            }
            Some(_) => return false,
            None => mask &= !Self::SECOND_BIT,
        }

        // Validate mask before touching any data.
        let mut probe = self.clone();
        if !probe.set_mask(mask) {
            return false;
        }

        // Mask is valid — commit both writes atomically.
        if let Some(sub) = pending_subyear {
            self.set_subyear(sub);
        }
        self.set_mask(mask);
        true
    }
}

impl std::fmt::Debug for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Timestamp")
            .field("year", &self.get_year())
            .field("month", &self.get_month())
            .field("day", &self.get_day())
            .field("hour", &self.get_hour())
            .field("minute", &self.get_minute())
            .field("second", &self.get_second())
            .field("mask", &format_args!("{:06b}", self.get_mask()))
            .field("packed", &format_args!("0x{:016X}", self.0))
            .finish()
    }
}

#[derive(Clone, Debug)]
pub enum Temporal {
    Timestamp(Timestamp),
    Tense(Tense),
    Period { start: Timestamp, end: Timestamp },
    Always,
}

// ── BeliefSource ──────────────────────────────────────────────────────────────
//
// Degrades during decay: Agent(id) → Inferred → entry dropped.
// Entries from Myself are never overridden by external merge_payload.

#[derive(Clone, Debug)]
pub enum BeliefSource {
    Myself,
    Agent(AgentId),
    Inferred,
}

// ── BeliefEntry ───────────────────────────────────────────────────────────────

pub struct BeliefEntry {
    pub state: Box<dyn State>,
    pub certainty: u8, // 0..=255
    pub probability: Probability,
    pub source: BeliefSource,
    pub temporal: Temporal,
}

impl BeliefEntry {
    pub fn clone_entry(&self) -> BeliefEntry {
        BeliefEntry {
            state: self.state.clone_box(),
            certainty: self.certainty,
            probability: self.probability.clone(),
            source: self.source.clone(),
            temporal: self.temporal.clone(),
        }
    }
}

// ── BeliefMap (all entries for one subject) ───────────────────────────────────

pub struct BeliefMap(pub HashMap<TypeId, Vec<BeliefEntry>>);

impl BeliefMap {
    pub fn new() -> Self {
        BeliefMap(HashMap::new())
    }

    pub fn insert<S: State + 'static>(&mut self, entry: BeliefEntry) {
        self.0.entry(TypeId::of::<S>()).or_default().push(entry);
    }

    // TODO: check if partial_cmp is really needed and add time search
    pub fn get<S: State + 'static>(&self) -> Option<&BeliefEntry> {
        self.0.get(&TypeId::of::<S>()).and_then(|v| {
            v.iter()
                .max_by(|a, b| a.certainty.partial_cmp(&b.certainty).unwrap())
        })
    }

    pub fn get_all<S: State + 'static>(&self) -> &[BeliefEntry] {
        self.0
            .get(&TypeId::of::<S>())
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn set<S: State + 'static>(&mut self, entries: Vec<BeliefEntry>) {
        self.0.insert(TypeId::of::<S>(), entries);
    }
}

// ── NestedBelief ──────────────────────────────────────────────────────────────
//
// Enables theory of mind: store "what I believe agent X believes" as a State.
//
// Example:
//   my beliefs["agent:1"] → BeliefMap → NestedBelief {
//       store: { "key1" → [BeliefEntry(InBox, certainty=255)] }
//   }
// Meaning: "I believe agent 1 believes key1 is in a box."
//
// Nesting is unbounded in structure but agents naturally shallow this by
// treating deeply nested beliefs with very low certainty.

#[derive(Clone)]
pub struct NestedBelief {
    pub store: BeliefStore,
}

impl NestedBelief {
    pub fn new() -> Self {
        NestedBelief {
            store: BeliefStore::new(),
        }
    }
}

impl State for NestedBelief {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn clone_box(&self) -> Box<dyn State> {
        Box::new(self.clone())
    }
}

// ── BeliefStore ───────────────────────────────────────────────────────────────

pub struct BeliefStore(pub HashMap<String, BeliefMap>);

impl BeliefStore {
    pub fn new() -> Self {
        BeliefStore(HashMap::new())
    }

    pub fn get(&self, key: &impl BeliefKey) -> Option<&BeliefMap> {
        self.0.get(key.to_key().as_ref())
    }

    pub fn get_mut(&mut self, key: &impl BeliefKey) -> Option<&mut BeliefMap> {
        self.0.get_mut(key.to_key().as_ref())
    }

    pub fn get_or_insert(&mut self, key: &impl BeliefKey) -> &mut BeliefMap {
        self.0
            .entry(key.to_key().into_owned())
            .or_insert_with(BeliefMap::new)
    }
}

impl Clone for BeliefStore {
    fn clone(&self) -> Self {
        let mut map = HashMap::new();
        for (key, bmap) in &self.0 {
            let mut new_bmap = BeliefMap::new();
            for (tid, entries) in &bmap.0 {
                new_bmap
                    .0
                    .insert(*tid, entries.iter().map(|e| e.clone_entry()).collect());
            }
            map.insert(key.clone(), new_bmap);
        }
        BeliefStore(map)
    }
}

// ── BeliefKey ─────────────────────────────────────────────────────────────────

pub trait BeliefKey {
    fn to_key(&self) -> Cow<'_, str>;
}

impl BeliefKey for AgentId {
    fn to_key(&self) -> Cow<'_, str> {
        Cow::Owned(format!("agent:{}", self.0))
    }
}

impl<'a> BeliefKey for &'a str {
    fn to_key(&self) -> Cow<'_, str> {
        Cow::Borrowed(self)
    }
}

impl BeliefKey for String {
    fn to_key(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.as_str())
    }
}

impl<T: BeliefKey + ?Sized> BeliefKey for &T {
    fn to_key(&self) -> Cow<'_, str> {
        (*self).to_key()
    }
}

// ── StateRegistry ─────────────────────────────────────────────────────────────
//
// Tracks semantic relationships between state types:
//   alias:     TypeId A → TypeId B  ("hungry" is another word for "needs_food")
//   composite: TypeId A → [TypeId]  ("starving" = high hunger AND low health)
//
// Used during belief merging to detect overlap and during display.
// Does not enforce merging automatically — the Agent decides.

pub struct StateRegistry {
    aliases: HashMap<TypeId, TypeId>,
    composites: HashMap<TypeId, Vec<TypeId>>,
    labels: HashMap<TypeId, &'static str>,
}

impl StateRegistry {
    pub fn new() -> Self {
        StateRegistry {
            aliases: HashMap::new(),
            composites: HashMap::new(),
            labels: HashMap::new(),
        }
    }

    pub fn register<S: State + 'static>(&mut self, label: &'static str) {
        self.labels.insert(TypeId::of::<S>(), label);
    }

    /// Declare A is an alias for B. During resolution, an A entry may be
    /// semantically merged with a B entry.
    pub fn alias<A: State + 'static, B: State + 'static>(&mut self) {
        self.aliases.insert(TypeId::of::<A>(), TypeId::of::<B>());
    }

    pub fn composite<A: State + 'static>(&mut self, components: Vec<TypeId>) {
        self.composites.insert(TypeId::of::<A>(), components);
    }

    pub fn canonical(&self, tid: TypeId) -> TypeId {
        *self.aliases.get(&tid).unwrap_or(&tid)
    }

    pub fn label(&self, tid: TypeId) -> Option<&'static str> {
        self.labels.get(&tid).copied()
    }
}

// ── Message ───────────────────────────────────────────────────────────────────

pub struct Message {
    pub from: AgentId,
    pub to: AgentId,
    pub payload: BeliefStore,
}

pub trait MessageCodec {
    fn encode(&self, msg: &Message) -> String;
    fn decode(&self, s: &str, from: AgentId, to: AgentId) -> Option<Message>;
}

// ── Utterance ─────────────────────────────────────────────────────────────────

pub trait IntoUtterance {
    fn to_utterance(&self) -> String;
}

pub trait FromUtterance: Sized {
    fn from_utterance(s: &str) -> Option<Self>;
}

// ── Agent ─────────────────────────────────────────────────────────────────────

pub trait Agent {
    fn id(&self) -> AgentId;
    fn states(&self) -> &StateMap;
    fn states_mut(&mut self) -> &mut StateMap;
    fn beliefs(&self) -> &BeliefStore;
    fn beliefs_mut(&mut self) -> &mut BeliefStore;

    fn on_message(&mut self, msg: Message) -> Vec<Message>;

    /// Apply memory decay. Default: -38 certainty per entry, degrade sources,
    /// drop zeroes. Override for custom decay strategies.
    fn decay(&mut self) {
        for (_, bmap) in self.beliefs_mut().0.iter_mut() {
            for (_, entries) in bmap.0.iter_mut() {
                for e in entries.iter_mut() {
                    e.certainty = e.certainty.saturating_sub(38);
                    // Source degrades as certainty fades: Agent → Inferred
                    if e.certainty < 102 {
                        if let BeliefSource::Agent(_) = e.source {
                            e.source = BeliefSource::Inferred;
                        }
                    }
                }
                entries.retain(|e| e.certainty > 0);
            }
        }
    }

    /// Report whether a claim from `agent` turned out to be correct.
    /// Default: no-op. Override to update trust scores.
    fn observe_outcome(&mut self, _agent: AgentId, _confirmed: bool) {}

    /// Conflict resolution. Default: append incoming, drop lower-certainty
    /// same-type entries. Override for trust-weighted merging.
    fn resolve_belief(
        &self,
        key: &str,
        from: AgentId,
        existing: Vec<BeliefEntry>,
        incoming: BeliefEntry,
    ) -> Vec<BeliefEntry> {
        let _ = (key, from);
        let incoming_tid = incoming.state.as_any().type_id();
        let incoming_cert = incoming.certainty;
        let mut result: Vec<BeliefEntry> = existing
            .into_iter()
            .filter(|e| {
                !(e.state.as_any().type_id() == incoming_tid && incoming_cert >= e.certainty)
            })
            .collect();
        result.push(incoming);
        result
    }

    /// Merge a BeliefStore payload into self.beliefs, calling resolve_belief
    /// for each entry. Entries originally from Myself are never overwritten.
    fn merge_payload(&mut self, from: AgentId, mut payload: BeliefStore) {
        let work: Vec<(String, TypeId, Vec<BeliefEntry>)> = payload
            .0
            .iter_mut()
            .flat_map(|(key, bmap)| {
                bmap.0
                    .drain()
                    .map(|(tid, entries)| (key.clone(), tid, entries))
                    .collect::<Vec<_>>()
            })
            .collect();

        for (key, tid, incoming_entries) in work {
            for ientry in incoming_entries {
                // Guard: never overwrite self-originated beliefs.
                let protected = self
                    .beliefs()
                    .0
                    .get(&key)
                    .and_then(|bm| bm.0.get(&tid))
                    .map(|v| v.iter().any(|e| matches!(e.source, BeliefSource::Myself)))
                    .unwrap_or(false);

                if protected {
                    continue;
                }

                let existing = self
                    .beliefs_mut()
                    .0
                    .entry(key.clone())
                    .or_insert_with(BeliefMap::new)
                    .0
                    .remove(&tid)
                    .unwrap_or_default();

                let merged = self.resolve_belief(&key, from, existing, ientry);

                self.beliefs_mut()
                    .0
                    .entry(key.clone())
                    .or_insert_with(BeliefMap::new)
                    .0
                    .insert(tid, merged);
            }
        }
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

pub trait World {
    fn codec(&self) -> Option<impl MessageCodec>;
    fn agents(&self) -> &HashMap<AgentId, Box<dyn Agent>>;
    fn agents_mut(&mut self) -> &mut HashMap<AgentId, Box<dyn Agent>>;

    fn dispatch(&mut self, initial: Message) {
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(initial);
        while let Some(msg) = queue.pop_front() {
            let Some(recipient) = self.agents_mut().get_mut(&msg.to) else {
                continue;
            };
            let responses = recipient.on_message(msg);
            queue.extend(responses);
        }
    }

    fn dispatch_from_str(&mut self, s: &str, from: AgentId, to: AgentId) -> bool {
        let Some(codec) = self.codec() else {
            return false;
        };
        let Some(msg) = codec.decode(s, from, to) else {
            return false;
        };
        drop(codec);
        self.dispatch(msg);
        true
    }
}
