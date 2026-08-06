//! MOEX trading calendar - market-hours and holiday awareness.
//!
//! Default trading session: 10:00–18:45 Moscow time (UTC+3), Mon–Fri.
//! Recurring public holidays are a minimal hardcoded set; callers may extend
//! via [`TradingCalendar::with_extra_holidays`] or load exact closure dates
//! (`YYYY-MM-DD`) from an external file via [`TradingCalendar::from_holiday_file`].
//!
//! Every order path should gate on [`TradingCalendar::is_open`] before placing
//! a trade - the bot otherwise runs 24/7 and piles up rejected orders on
//! evenings, weekends and holidays.

use std::path::Path;

use anyhow::Context;
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Timelike, Utc, Weekday};

/// MOEX regular trading session (Moscow time, UTC+3).
pub const MOEX_OPEN_HOUR: u32 = 10;
pub const MOEX_CLOSE_HOUR: u32 = 18;
pub const MOEX_CLOSE_MINUTE: u32 = 45;

/// Moscow timezone offset in hours (no DST in Russia since 2014).
const MSK_OFFSET_HOURS: i64 = 3;

#[derive(Debug, Clone)]
pub struct TradingCalendar {
    /// Recurring annual non-trading days (month, day) in MSK calendar terms.
    /// Minimal hardcoded set - covers the main MOEX closures. Upgrade path:
    /// load from an external holidays feed when available.
    recurring: Vec<(u32, u32)>,
    /// Exact closure dates (e.g. postponed weekends / substituted holidays)
    /// loaded from an external file or feed.
    one_off: Vec<NaiveDate>,
}

impl Default for TradingCalendar {
    fn default() -> Self {
        // Minimal set of recurring MOEX public holidays (month, day).
        // Russia has no DST, so these are stable year over year.
        Self {
            recurring: vec![
                (1, 1),  // New Year
                (1, 2),  // New Year holidays
                (1, 3),  // New Year holidays
                (1, 4),  // New Year holidays
                (1, 5),  // New Year holidays
                (1, 6),  // New Year holidays
                (1, 7),  // Orthodox Christmas
                (1, 8),  // New Year holidays
                (2, 23), // Defender of the Fatherland Day
                (3, 8),  // International Women's Day
                (5, 1),  // Labour Day
                (5, 9),  // Victory Day
                (6, 12), // Russia Day
                (11, 4), // Unity Day
            ],
            one_off: Vec::new(),
        }
    }
}

/// Parse a holidays file into exact closure dates.
///
/// Expected format: one `YYYY-MM-DD` date per line; `#` starts a comment;
/// blank lines are ignored. Duplicates are removed.
fn load_holiday_dates(path: &Path) -> anyhow::Result<Vec<NaiveDate>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read holidays file {}", path.display()))?;
    let mut dates = Vec::new();
    for (line_no, raw) in content.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let date = NaiveDate::parse_from_str(line, "%Y-%m-%d").with_context(|| {
            format!(
                "Invalid holiday date {:?} at {}:{}",
                line,
                path.display(),
                line_no + 1
            )
        })?;
        dates.push(date);
    }
    dates.sort_unstable();
    dates.dedup();
    Ok(dates)
}

impl TradingCalendar {
    /// Add extra recurring holiday dates (month, day) beyond the built-in set.
    pub fn with_extra_holidays(mut self, extra: Vec<(u32, u32)>) -> Self {
        self.recurring.extend(extra);
        self.recurring.sort_unstable();
        self.recurring.dedup();
        self
    }

    /// Add exact one-off closure dates (`YYYY-MM-DD`), e.g. substituted
    /// weekends or moved public holidays for the current year.
    pub fn with_one_off_holidays(mut self, dates: Vec<NaiveDate>) -> Self {
        self.one_off.extend(dates);
        self.one_off.sort_unstable();
        self.one_off.dedup();
        self
    }

    /// Load a `TradingCalendar` from an external holidays file, merged on top
    /// of the built-in recurring set. See [`load_holiday_dates`] for format.
    ///
    /// If the file does not exist, the built-in calendar is returned unchanged
    /// so that a missing feed never disables trading-hour gating.
    pub fn from_holiday_file(path: &Path) -> Self {
        match load_holiday_dates(path) {
            Ok(dates) => Self::default().with_one_off_holidays(dates),
            Err(e) => {
                log::warn!("Holidays file ignored: {:#}", e);
                Self::default()
            }
        }
    }

    /// Convert a UTC instant to Moscow civil date + HMS.
    fn msk_components(utc: DateTime<Utc>) -> (NaiveDate, u32, u32, u32, Weekday) {
        let msk = utc + chrono::Duration::hours(MSK_OFFSET_HOURS);
        (
            msk.date_naive(),
            msk.hour(),
            msk.minute(),
            msk.second(),
            msk.weekday(),
        )
    }

    /// True if the MOEX regular session is open at the given UTC time.
    pub fn is_open(&self, at: DateTime<Utc>) -> bool {
        let (date, hour, minute, _, weekday) = Self::msk_components(at);

        // Weekend - MOEX is closed Sat/Sun.
        if matches!(weekday, Weekday::Sat | Weekday::Sun) {
            return false;
        }

        // Public holiday (recurring month/day or exact date).
        if self.recurring.contains(&(date.month(), date.day())) || self.one_off.contains(&date) {
            return false;
        }

        // Trading hours 10:00–18:45 MSK (inclusive of open, exclusive of close minute boundary).
        let after_open = hour >= MOEX_OPEN_HOUR;
        let before_close =
            hour < MOEX_CLOSE_HOUR || (hour == MOEX_CLOSE_HOUR && minute < MOEX_CLOSE_MINUTE);

        after_open && before_close
    }

    /// Convenience: is MOEX open right now?
    pub fn is_open_now(&self) -> bool {
        self.is_open(Utc::now())
    }

    /// Next UTC instant at or after `at` when the market opens.
    /// Bounded scan - at most ~3 calendar days of iteration.
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
        Utc.timestamp_opt(at.timestamp() + 60 * 60, 0)
            .single()
            .unwrap_or(at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monday 2026-02-09 09:00 MSK = 06:00 UTC - before open.
    #[test]
    fn monday_before_open_is_closed() {
        let cal = TradingCalendar::default();
        // 2026-02-09T06:00:00Z = Monday 09:00 MSK
        let t = Utc.with_ymd_and_hms(2026, 2, 9, 6, 0, 0).unwrap();
        assert!(!cal.is_open(t));
    }

    /// Monday 2026-02-09 12:00 MSK = 09:00 UTC - within session.
    #[test]
    fn midday_weekday_is_open() {
        let cal = TradingCalendar::default();
        let t = Utc.with_ymd_and_hms(2026, 2, 9, 9, 0, 0).unwrap();
        assert!(cal.is_open(t));
    }

    /// Monday 2026-02-09 19:00 MSK = 16:00 UTC - after close.
    #[test]
    fn after_close_is_closed() {
        let cal = TradingCalendar::default();
        let t = Utc.with_ymd_and_hms(2026, 2, 9, 16, 0, 0).unwrap();
        assert!(!cal.is_open(t));
    }

    /// Saturday - always closed.
    #[test]
    fn weekend_is_closed() {
        let cal = TradingCalendar::default();
        // 2026-02-14 is Saturday; 12:00 MSK = 09:00 UTC
        let t = Utc.with_ymd_and_hms(2026, 2, 14, 9, 0, 0).unwrap();
        assert!(!cal.is_open(t));
    }

    /// New Year's Day - closed even though it's a Friday in 2026.
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
        // 2026-07-15 12:00 MSK = 09:00 UTC, Wednesday - normally open
        let t = Utc.with_ymd_and_hms(2026, 7, 15, 9, 0, 0).unwrap();
        assert!(!cal.is_open(t));
    }

    /// One-off exact date (e.g. a substituted working weekend) closes the market
    /// even though the weekday would normally be open.
    #[test]
    fn one_off_holiday_closes_single_date() {
        let holiday = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
        let cal = TradingCalendar::default().with_one_off_holidays(vec![holiday]);
        // Wednesday 12:00 MSK = 09:00 UTC - normally open.
        let t = Utc.with_ymd_and_hms(2026, 7, 15, 9, 0, 0).unwrap();
        assert!(!cal.is_open(t));
        // The following day must stay open.
        let t2 = Utc.with_ymd_and_hms(2026, 7, 16, 9, 0, 0).unwrap();
        assert!(cal.is_open(t2));
    }

    #[test]
    fn holidays_file_parses_dates() {
        let dir = std::env::temp_dir().join(format!("holidays_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("holidays.txt");
        std::fs::write(
            &path,
            "# MOEX closures 2026\n2026-05-11\n2026-05-12\n\n2026-05-11\n",
        )
        .unwrap();
        let dates = load_holiday_dates(&path).unwrap();
        assert_eq!(
            dates,
            vec![
                NaiveDate::from_ymd_opt(2026, 5, 11).unwrap(),
                NaiveDate::from_ymd_opt(2026, 5, 12).unwrap(),
            ]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Missing holidays file must not break trading-hour gating - the built-in
    /// calendar is used instead.
    #[test]
    fn missing_holidays_file_falls_back_to_default() {
        let cal = TradingCalendar::from_holiday_file(Path::new("/nonexistent/holidays.txt"));
        let t = Utc.with_ymd_and_hms(2026, 2, 9, 9, 0, 0).unwrap();
        assert!(cal.is_open(t));
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
