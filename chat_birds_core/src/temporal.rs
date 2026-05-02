use std::cmp::Ordering;

/// Temporal perspectives for beliefs.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum TenseTime {
    Past,
    Present,
    Future,
}

/// Packed 64-bit timestamp with compact field representation.
///
/// Encodes up to 6 fields (year, month, day, hour, minute, second) in a single u64.
/// Only contiguous field masks are valid (no sparse combinations like year + day without month).
///
/// Layout:
/// - Top 5 bits: shape index (0..21), encodes which fields are present
/// - Middle 34 bits: year with offset (2^33 year range)
/// - Bottom 25 bits: sub-year encoding (month/day/hour/minute/second mixed-radix)
///
/// A fully specified timestamp uses 12×31×24×60×60 = 32_140_800 states (25 bits).
/// This leaves room for a massive year range (~550 billion years representable).
#[derive(Clone, PartialEq, PartialOrd)]
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

    /// Create an empty timestamp with no fields set.
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

    /// Get the field presence mask (6-bit pattern indicating which fields are set).
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

    /// Set the field presence mask. Returns false if the mask is invalid (sparse).
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

/// Temporal context for a belief: when it was true, how long, or always/never.
#[derive(Clone, Debug)]
pub enum Temporal {
    Timestamp(Timestamp),
    Tense(TenseTime),
    Period {
        start: Box<Temporal>,
        end: Box<Temporal>,
    },
    Always,
}

impl PartialEq for Temporal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Temporal::Always, Temporal::Always) => true,

            (Temporal::Timestamp(a), Temporal::Timestamp(b)) => a.0 == b.0,

            (Temporal::Tense(a), Temporal::Tense(b)) => a == b,

            (Temporal::Period { start: s1, end: e1 }, Temporal::Period { start: s2, end: e2 }) => {
                **s1 == **s2 && **e1 == **e2
            }

            // A period with identical bounds equals the bound itself (via recursive comparison).
            (Temporal::Period { start, end }, other) | (other, Temporal::Period { start, end }) => {
                **start == **end && **start == *other
            }

            // Different variants or non-matching bounds are not equal.
            _ => false,
        }
    }
}

impl PartialOrd for Temporal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else {
            match (self, other) {
                (Temporal::Timestamp(a), Temporal::Timestamp(b)) => a.partial_cmp(b),
                (Temporal::Tense(a), Temporal::Tense(b)) => match (a, b) {
                    (TenseTime::Past, TenseTime::Past) => Some(Ordering::Equal),
                    (TenseTime::Present, TenseTime::Present) => Some(Ordering::Equal),
                    (TenseTime::Future, TenseTime::Future) => Some(Ordering::Equal),
                    (TenseTime::Past, TenseTime::Present) => Some(Ordering::Less),
                    (TenseTime::Present, TenseTime::Future) => Some(Ordering::Less),
                    (TenseTime::Past, TenseTime::Future) => Some(Ordering::Less),
                    (TenseTime::Present, TenseTime::Past) => Some(Ordering::Greater),
                    (TenseTime::Future, TenseTime::Present) => Some(Ordering::Greater),
                    (TenseTime::Future, TenseTime::Past) => Some(Ordering::Greater),
                },
                (
                    Temporal::Period { start: s1, end: e1 },
                    Temporal::Period { start: s2, end: e2 },
                ) => {
                    // Check if s2 <= e2 <= s1 <= e1 (B contained/before A, so A > B)
                    let s2_le_e2 =
                        matches!(s2.partial_cmp(e2), Some(Ordering::Less | Ordering::Equal));
                    let e2_le_s1 =
                        matches!(e2.partial_cmp(s1), Some(Ordering::Less | Ordering::Equal));
                    let s1_le_e1 =
                        matches!(s1.partial_cmp(e1), Some(Ordering::Less | Ordering::Equal));

                    if s2_le_e2 && e2_le_s1 && s1_le_e1 {
                        return Some(Ordering::Greater);
                    }

                    // Check if s1 <= e1 <= s2 <= e2 (A contained/before B, so A < B)
                    let e1_le_s2 =
                        matches!(e1.partial_cmp(s2), Some(Ordering::Less | Ordering::Equal));

                    if s1_le_e1 && e1_le_s2 && s2_le_e2 {
                        return Some(Ordering::Less);
                    }

                    None
                }
                (Temporal::Period { start, end }, Temporal::Timestamp(t)) => {
                    // Check if period is valid: start <= end
                    let s_le_e = matches!(
                        start.as_ref().partial_cmp(end.as_ref()),
                        Some(Ordering::Less | Ordering::Equal)
                    );
                    if !s_le_e {
                        return None;
                    }

                    let t_temporal = Temporal::Timestamp(t.clone());
                    // Check if t < s: if true, period > timestamp
                    let t_before_s = matches!(
                        start.as_ref().partial_cmp(&t_temporal),
                        Some(Ordering::Greater)
                    );
                    if t_before_s {
                        return Some(Ordering::Greater);
                    }

                    // Check if t > e: if true, period < timestamp
                    let t_after_e =
                        matches!(end.as_ref().partial_cmp(&t_temporal), Some(Ordering::Less));
                    if t_after_e {
                        return Some(Ordering::Less);
                    }

                    None
                }
                (Temporal::Timestamp(t), Temporal::Period { start, end }) => {
                    let s_le_e = matches!(
                        start.as_ref().partial_cmp(end.as_ref()),
                        Some(Ordering::Less | Ordering::Equal)
                    );
                    if !s_le_e {
                        return None;
                    }

                    let t_temporal = Temporal::Timestamp(t.clone());
                    let t_before_s = matches!(
                        start.as_ref().partial_cmp(&t_temporal),
                        Some(Ordering::Greater)
                    );
                    if t_before_s {
                        return Some(Ordering::Less);
                    }

                    let t_after_e =
                        matches!(end.as_ref().partial_cmp(&t_temporal), Some(Ordering::Less));
                    if t_after_e {
                        return Some(Ordering::Greater);
                    }

                    None
                }
                (Temporal::Period { start, end }, Temporal::Tense(t)) => {
                    // Check if period is valid: start <= end
                    let s_le_e = matches!(
                        start.as_ref().partial_cmp(end.as_ref()),
                        Some(Ordering::Less | Ordering::Equal)
                    );
                    if !s_le_e {
                        return None;
                    }

                    let t_temporal = Temporal::Tense(*t);
                    // Check if t < s: if true, period > tense
                    let t_before_s = matches!(
                        start.as_ref().partial_cmp(&t_temporal),
                        Some(Ordering::Greater)
                    );
                    if t_before_s {
                        return Some(Ordering::Greater);
                    }

                    // Check if t > e: if true, period < tense
                    let t_after_e =
                        matches!(end.as_ref().partial_cmp(&t_temporal), Some(Ordering::Less));
                    if t_after_e {
                        return Some(Ordering::Less);
                    }

                    None
                }
                (Temporal::Tense(t), Temporal::Period { start, end }) => {
                    let s_le_e = matches!(
                        start.as_ref().partial_cmp(end.as_ref()),
                        Some(Ordering::Less | Ordering::Equal)
                    );
                    if !s_le_e {
                        return None;
                    }

                    let t_temporal = Temporal::Tense(*t);
                    let t_before_s = matches!(
                        start.as_ref().partial_cmp(&t_temporal),
                        Some(Ordering::Greater)
                    );
                    if t_before_s {
                        return Some(Ordering::Less);
                    }

                    let t_after_e =
                        matches!(end.as_ref().partial_cmp(&t_temporal), Some(Ordering::Less));
                    if t_after_e {
                        return Some(Ordering::Greater);
                    }

                    None
                }
                _ => None,
            }
        }
    }
}

pub struct Clock(pub Box<dyn Fn() -> Timestamp + Send + Sync>);

impl Clock {
    pub fn now(&self) -> Timestamp {
        (self.0)()
    }

    pub fn system() -> Self {
        Clock(Box::new(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Unix epoch = 1970, encode as year offset
            let year = 1970 + secs / 31_536_000;
            let rem = secs % 31_536_000;
            let month = (rem / 2_592_000) as u8 + 1;
            let rem = rem % 2_592_000;
            let day = (rem / 86_400) as u8 + 1;
            let rem = rem % 86_400;
            let hour = (rem / 3_600) as u8;
            let rem = rem % 3_600;
            let minute = (rem / 60) as u8;
            let second = (rem % 60) as u8;

            let mut t = Timestamp::empty();
            t.set_year(Some(year));
            t.set_month(Some(month));
            t.set_day(Some(day));
            t.set_hour(Some(hour));
            t.set_minute(Some(minute));
            t.set_second(Some(second));
            t
        }))
    }

    /// Game/sim clock — user supplies tick→Timestamp conversion.
    pub fn custom(f: impl Fn() -> Timestamp + Send + Sync + 'static) -> Self {
        Clock(Box::new(f))
    }

    /// Compare a Temporal against "now" with a scope tolerance.
    ///
    /// First tries standard `partial_cmp`. If that returns `None` (incomparable),
    /// uses the scope to define a "Present" window: [now - scope.0, now + scope.0].
    ///
    /// For Timestamps: returns `Equal` if within scope, `Less` if before, `Greater` if after.
    /// For Periods: returns `Equal` if both bounds fit within scope, else `Less`/`Greater`/`None`.
    /// For other Temporals: returns `None` if not directly comparable.
    pub fn cmp_temporal_with_scope(
        &self,
        temporal: &Temporal,
        scope: &Timestamp,
    ) -> Option<Ordering> {
        let now = self.now();

        // Use scope first to define Present around now.
        let scope_val = scope.0;
        let now_lower = Timestamp(now.0.saturating_sub(scope_val));
        let now_upper = Timestamp(now.0.saturating_add(scope_val));

        match temporal {
            Temporal::Timestamp(t) => {
                // Check if t fits within [now_lower, now_upper].
                if t.0 >= now_lower.0 && t.0 <= now_upper.0 {
                    Some(Ordering::Equal) // Within scope → "Present"
                } else if t.0 < now_lower.0 {
                    Some(Ordering::Less) // Before scope → "Past"
                } else {
                    Some(Ordering::Greater) // After scope → "Future"
                }
            }
            Temporal::Period { start, end } => {
                // Try to extract timestamps from both bounds.
                if let (Temporal::Timestamp(s), Temporal::Timestamp(e)) =
                    (start.as_ref(), end.as_ref())
                {
                    // Check if both bounds fit within [now_lower, now_upper].
                    if s.0 >= now_lower.0 && e.0 <= now_upper.0 {
                        Some(Ordering::Equal) // Both bounds within scope
                    } else if e.0 < now_lower.0 {
                        Some(Ordering::Less) // Period ends before scope
                    } else if s.0 > now_upper.0 {
                        Some(Ordering::Greater) // Period starts after scope
                    } else {
                        // Scope cannot decide for overlap; fall back to normal comparison.
                        temporal.partial_cmp(&Temporal::Timestamp(now.clone()))
                    }
                } else {
                    // Non-timestamp period bounds: fall back to normal comparison.
                    temporal.partial_cmp(&Temporal::Timestamp(now.clone()))
                }
            }
            // Other variants: let regular temporal ordering decide.
            _ => temporal.partial_cmp(&Temporal::Timestamp(now)),
        }
    }
}
