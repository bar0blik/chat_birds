/// Temporal perspectives for beliefs.
#[derive(Clone, Debug)]
pub enum Tense {
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
    Tense(Tense),
    Period { start: Timestamp, end: Timestamp },
    Always,
}
