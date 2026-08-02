//! MOEX trading calendar — market-hours and holiday awareness.
//!
//! Default trading session: 10:00–18:45 Moscow time (UTC+3), Mon–Fri.
//! Holidays are a minimal hardcoded set for the current year; callers may
//! extend via [`TradingCalendar::with_extra_holidays`].
//!
//! Every order path should gate on [`TradingCalendar::is_open`] before placing
//! a trade — the bot otherwise runs 24/7 and piles up rejected orders on
//! evenings, weekends and holidays.

use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc, Weekday};

/// MOEX regular trading session (Moscow time, UTC+3).
pub const MOEX_OPEN_HOUR: u32 = 10;
pub const MOEX_OPEN_MINUTE: u32 = 0;
pub const MOEX_CLOSE_HOUR: u32 = 18;
pub const MOEX_CLOSE_MINUTE: u32 = 45;

/// Moscow timezone offset in hours (no DST in Russia since 2014).
const MSK_OFFSET_HOURS: i64 = 3;

#[derive(Debug, Clone)]
pub struct TradingCalendar {
    /// Extra non-trading days (month, day) in UTC date terms of the MSK calendar.
    /// ponytail: minimal hardcoded holiday set — covers the main MOEX closures.
    /// Upgrade path: load from an external holidays feed when available.
    holidays: Vec<(u32, u32)>,
}

impl Default for TradingCalendar {
    fn default() -> Self {
        // Minimal set of recurring MOEX public holidays (month, day).
        // Russia has no DST, so these are stable year over year.
        Self {
            holidays: vec![
                (1, 1),   // New Year
                (1, 2),   // New Year holidays
                (1, 3),   // New Year holidays
                (1, 4),   // New Year holidays
                (1, 5),   // New Year holidays
                (1, 6),   // New Year holidays
                (1, 7),   // Orthodox Christmas
                (1, 8),   // New Year holidays
                (2, 23),  // Defender of the Fatherland Day
                (3, 8),   // International Women's Day
                (5, 1),   // Labour Day
                (5, 9),   // Victory Day
                (6, 12),  // Russia Day
                (11, 4),  // Unity Day
            ],
        }
    }
}

impl TradingCalendar {
    /// Add extra holiday dates (month, day) beyond the built-in set.
    pub fn with_extra_holidays(mut self, extra: Vec<(u32, u32)>) -> Self {
        self.holidays.extend(extra);
        self.holidays.sort_unstable();
        self.holidays.dedup();
        self
    }

    /// Convert a UTC instant to Moscow civil date + HMS.
    fn msk_components(utc: DateTime<Utc>) -> (i32, u32, u32, u32, u32, u32, Weekday) {
        let msk = utc + chrono::Duration::hours(MSK_OFFSET_HOURS);
        (
            msk.year(),
            msk.month(),
            msk.day(),
            msk.hour(),
            msk.minute(),
            msk.second(),
            msk.weekday(),
        )
    }

    /// True if the MOEX regular session is open at the given UTC time.
    pub fn is_open(&self, at: DateTime<Utc>) -> bool {
        let (_, month, day, hour, minute, _, weekday) = Self::msk_components(at);

        // Weekend — MOEX is closed Sat/Sun.
        if matches!(weekday, Weekday::Sat | Weekday::Sun) {
            return false;
        }

        // Public holiday.
        if self.holidays.contains(&(month, day)) {
            return false;
        }

        // Trading hours 10:00–18:45 MSK (inclusive of open, exclusive of close minute boundary).
        let after_open = hour > MOEX_OPEN_HOUR
            || (hour == MOEX_OPEN_HOUR && minute >= MOEX_OPEN_MINUTE);
        let before_close = hour < MOEX_CLOSE_HOUR
            || (hour == MOEX_CLOSE_HOUR && minute < MOEX_CLOSE_MINUTE);

        after_open && before_close
    }

    /// Convenience: is MOEX open right now?
    pub fn is_open_now(&self) -> bool {
        self.is_open(Utc::now())
    }

    /// Next UTC instant at or after `at` when the market opens.
    /// Bounded scan — at most ~3 calendar days of iteration.
    pub fn next_open(&self, at: DateTime<Utc>) -> DateTime<Utc> {
        let mut t = at;
        // Scan forward in 1-minute steps until open, capped at 3 days.
        let limit = at + chrono::Duration::days(3);
        while t <= limit {
            if self.is_open(t) {
                return t;
            }
            t += chrono::Duration::minutes(1);
        }
        // Fallback: should not happen for a weekly calendar.
        Utc.timestamp_opt(at.timestamp() + 60 * 60, 0).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monday 2026-02-09 09:00 MSK = 06:00 UTC — before open.
    #[test]
    fn monday_before_open_is_closed() {
        let cal = TradingCalendar::default();
        // 2026-02-09T06:00:00Z = Monday 09:00 MSK
        let t = Utc.with_ymd_and_hms(2026, 2, 9, 6, 0, 0).unwrap();
        assert!(!cal.is_open(t));
    }

    /// Monday 2026-02-09 12:00 MSK = 09:00 UTC — within session.
    #[test]
    fn midday_weekday_is_open() {
        let cal = TradingCalendar::default();
        let t = Utc.with_ymd_and_hms(2026, 2, 9, 9, 0, 0).unwrap();
        assert!(cal.is_open(t));
    }

    /// Monday 2026-02-09 19:00 MSK = 16:00 UTC — after close.
    #[test]
    fn after_close_is_closed() {
        let cal = TradingCalendar::default();
        let t = Utc.with_ymd_and_hms(2026, 2, 9, 16, 0, 0).unwrap();
        assert!(!cal.is_open(t));
    }

    /// Saturday — always closed.
    #[test]
    fn weekend_is_closed() {
        let cal = TradingCalendar::default();
        // 2026-02-14 is Saturday; 12:00 MSK = 09:00 UTC
        let t = Utc.with_ymd_and_hms(2026, 2, 14, 9, 0, 0).unwrap();
        assert!(!cal.is_open(t));
    }

    /// New Year's Day — closed even though it's a Friday in 2026.
    #[test]
    fn new_year_holiday_is_closed() {
        let cal = TradingCalendar::default();
        // 2026-01-01 12:00 MSK = 09:00 UTC
        let t = Utc.with_ymd_and_hms(2026, 1, 1, 9, 0, 0).unwrap();
        assert!(!cal.is_open(t));
    }

    #[test]
    fn extra_holidays_added() {
        let cal = TradingCalendar::default().with_extra_holidays(vec![(7, 15)]);
        // 2026-07-15 12:00 MSK = 09:00 UTC, Wednesday — normally open
        let t = Utc.with_ymd_and_hms(2026, 7, 15, 9, 0, 0).unwrap();
        assert!(!cal.is_open(t));
    }

    #[test]
    fn next_open_finds_opening() {
        let cal = TradingCalendar::default();
        // Saturday 09:00 UTC → next open is Monday 07:00 UTC (10:00 MSK).
        let sat = Utc.with_ymd_and_hms(2026, 2, 14, 9, 0, 0).unwrap();
        let next = cal.next_open(sat);
        assert!(cal.is_open(next));
        // Monday is 2026-02-16.
        assert_eq!(next.weekday(), Weekday::Mon);
    }

    /// Open boundary: exactly 10:00 MSK is open.
    #[test]
    fn open_boundary_is_open() {
        let cal = TradingCalendar::default();
        // 10:00 MSK = 07:00 UTC on Monday 2026-02-09
        let t = Utc.with_ymd_and_hms(2026, 2, 9, 7, 0, 0).unwrap();
        assert!(cal.is_open(t));
    }

    /// Close boundary: exactly 18:45 MSK is closed (exclusive).
    #[test]
    fn close_boundary_is_closed() {
        let cal = TradingCalendar::default();
        // 18:45 MSK = 15:45 UTC on Monday 2026-02-09
        let t = Utc.with_ymd_and_hms(2026, 2, 9, 15, 45, 0).unwrap();
        assert!(!cal.is_open(t));
    }
}
