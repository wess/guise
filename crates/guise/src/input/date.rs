//! `Date` — a plain calendar date plus the math the date components need.
//!
//! Pure logic, no gpui: leap years, month lengths, weekday math, month grids
//! for calendar layouts, and a small token formatter/parser. Algorithms for
//! day-count conversion follow Howard Hinnant's civil-date derivations.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// English month names, January first.
pub const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Day of week. `index()` is 0-based from Sunday.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weekday {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Weekday {
    pub const ALL: [Weekday; 7] = [
        Weekday::Sunday,
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
    ];

    pub fn index(self) -> u32 {
        self as u32
    }

    pub fn from_index(index: u32) -> Weekday {
        Weekday::ALL[(index % 7) as usize]
    }

    pub fn name(self) -> &'static str {
        match self {
            Weekday::Sunday => "Sunday",
            Weekday::Monday => "Monday",
            Weekday::Tuesday => "Tuesday",
            Weekday::Wednesday => "Wednesday",
            Weekday::Thursday => "Thursday",
            Weekday::Friday => "Friday",
            Weekday::Saturday => "Saturday",
        }
    }

    /// Two-letter header label ("Su", "Mo", …).
    pub fn short(self) -> &'static str {
        &self.name()[..2]
    }
}

/// A calendar date. Construct with [`Date::new`], which validates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i32,
    month: u32,
    day: u32,
}

/// True for Gregorian leap years.
pub fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Number of days in the given month (1–12) of the given year.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Days since 1970-01-01 (negative before it).
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = i64::from(if month <= 2 { year - 1 } else { year });
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (i64::from(month) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of [`days_from_civil`].
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = (if month <= 2 { y + 1 } else { y }) as i32;
    (year, month, day)
}

impl Date {
    /// A validated date, or `None` for impossible ones (Feb 30, month 13, …).
    pub fn new(year: i32, month: u32, day: u32) -> Option<Date> {
        if (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month) {
            Some(Date { year, month, day })
        } else {
            None
        }
    }

    /// Today in UTC (std has no timezone database; a calendar highlight is
    /// the intended use, not civil timekeeping).
    pub fn today() -> Date {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Date::from_days(secs.div_euclid(86_400))
    }

    pub fn year(self) -> i32 {
        self.year
    }

    pub fn month(self) -> u32 {
        self.month
    }

    pub fn day(self) -> u32 {
        self.day
    }

    /// Days since 1970-01-01.
    pub fn to_days(self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    /// The date `days` after 1970-01-01.
    pub fn from_days(days: i64) -> Date {
        let (year, month, day) = civil_from_days(days);
        Date { year, month, day }
    }

    pub fn weekday(self) -> Weekday {
        Weekday::from_index((self.to_days() + 4).rem_euclid(7) as u32)
    }

    pub fn add_days(self, days: i64) -> Date {
        Date::from_days(self.to_days() + days)
    }

    /// Shift by whole months, clamping the day to the target month's length
    /// (Jan 31 + 1 month = Feb 28/29).
    pub fn add_months(self, months: i32) -> Date {
        let total = self.year * 12 + (self.month as i32 - 1) + months;
        let year = total.div_euclid(12);
        let month = (total.rem_euclid(12) + 1) as u32;
        let day = self.day.min(days_in_month(year, month));
        Date { year, month, day }
    }

    pub fn month_name(self) -> &'static str {
        MONTH_NAMES[(self.month - 1) as usize]
    }

    /// Render through a token pattern. Tokens: `YYYY`, `MM`, `M`, `DD`, `D`,
    /// `MMM` (Jan), `MMMM` (January). Anything else passes through.
    pub fn format(self, pattern: &str) -> String {
        let mut out = String::with_capacity(pattern.len() + 4);
        let bytes = pattern.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let run = |ch: u8| bytes[i..].iter().take_while(|&&b| b == ch).count();
            match bytes[i] {
                b'Y' => {
                    let n = run(b'Y');
                    out.push_str(&format!("{:04}", self.year));
                    i += n;
                }
                b'M' => match run(b'M') {
                    1 => {
                        out.push_str(&self.month.to_string());
                        i += 1;
                    }
                    2 => {
                        out.push_str(&format!("{:02}", self.month));
                        i += 2;
                    }
                    3 => {
                        out.push_str(&self.month_name()[..3]);
                        i += 3;
                    }
                    _ => {
                        out.push_str(self.month_name());
                        i += run(b'M');
                    }
                },
                b'D' => {
                    let n = run(b'D');
                    if n >= 2 {
                        out.push_str(&format!("{:02}", self.day));
                    } else {
                        out.push_str(&self.day.to_string());
                    }
                    i += n;
                }
                other => {
                    out.push(other as char);
                    i += 1;
                }
            }
        }
        out
    }

    /// Parse `"YYYY-MM-DD"` (also tolerates single-digit month/day).
    pub fn parse_iso(s: &str) -> Option<Date> {
        let mut parts = s.trim().split('-');
        let year: i32 = parts.next()?.parse().ok()?;
        let month: u32 = parts.next()?.parse().ok()?;
        let day: u32 = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Date::new(year, month, day)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The 42 cells (6 weeks × 7 days) a month calendar shows for `year`/`month`,
/// starting each week on `week_start`. Leading/trailing cells come from the
/// neighboring months; compare `.month()` against `month` to dim them.
pub fn month_grid(year: i32, month: u32, week_start: Weekday) -> Vec<Date> {
    let first = Date::new(year, month, 1).unwrap_or_else(|| Date::from_days(0));
    let lead = (first.weekday().index() + 7 - week_start.index()) % 7;
    let start = first.add_days(-i64::from(lead));
    (0..42).map(|i| start.add_days(i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leap_years() {
        assert!(is_leap_year(2024));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2026));
    }

    #[test]
    fn month_lengths() {
        assert_eq!(days_in_month(2026, 1), 31);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 13), 0);
    }

    #[test]
    fn validation() {
        assert!(Date::new(2026, 2, 29).is_none());
        assert!(Date::new(2024, 2, 29).is_some());
        assert!(Date::new(2026, 0, 1).is_none());
        assert!(Date::new(2026, 12, 31).is_some());
        assert!(Date::new(2026, 6, 0).is_none());
    }

    #[test]
    fn epoch_round_trip() {
        assert_eq!(Date::new(1970, 1, 1).unwrap().to_days(), 0);
        assert_eq!(Date::from_days(0), Date::new(1970, 1, 1).unwrap());
        for days in [-1_000_000, -365, -1, 0, 1, 365, 738_000, 1_000_000] {
            assert_eq!(Date::from_days(days).to_days(), days);
        }
    }

    #[test]
    fn known_weekdays() {
        // 1970-01-01 was a Thursday; 2026-07-14 is a Tuesday.
        assert_eq!(Date::new(1970, 1, 1).unwrap().weekday(), Weekday::Thursday);
        assert_eq!(Date::new(2026, 7, 14).unwrap().weekday(), Weekday::Tuesday);
        assert_eq!(Date::new(2000, 1, 1).unwrap().weekday(), Weekday::Saturday);
        assert_eq!(Date::new(1899, 12, 31).unwrap().weekday(), Weekday::Sunday);
    }

    #[test]
    fn add_days_crosses_boundaries() {
        let d = Date::new(2026, 12, 31).unwrap();
        assert_eq!(d.add_days(1), Date::new(2027, 1, 1).unwrap());
        assert_eq!(d.add_days(-365), Date::new(2025, 12, 31).unwrap());
        let leap = Date::new(2024, 2, 28).unwrap();
        assert_eq!(leap.add_days(1), Date::new(2024, 2, 29).unwrap());
        assert_eq!(leap.add_days(2), Date::new(2024, 3, 1).unwrap());
    }

    #[test]
    fn add_months_clamps() {
        let jan31 = Date::new(2026, 1, 31).unwrap();
        assert_eq!(jan31.add_months(1), Date::new(2026, 2, 28).unwrap());
        assert_eq!(jan31.add_months(13), Date::new(2027, 2, 28).unwrap());
        assert_eq!(jan31.add_months(-2), Date::new(2025, 11, 30).unwrap());
        let jul = Date::new(2026, 7, 14).unwrap();
        assert_eq!(jul.add_months(12), Date::new(2027, 7, 14).unwrap());
        assert_eq!(jul.add_months(-7), Date::new(2025, 12, 14).unwrap());
    }

    #[test]
    fn ordering() {
        let a = Date::new(2026, 7, 14).unwrap();
        let b = Date::new(2026, 7, 15).unwrap();
        let c = Date::new(2027, 1, 1).unwrap();
        assert!(a < b && b < c);
    }

    #[test]
    fn grid_starts_on_week_start() {
        // July 2026 starts on a Wednesday.
        let grid = month_grid(2026, 7, Weekday::Sunday);
        assert_eq!(grid.len(), 42);
        assert_eq!(grid[0], Date::new(2026, 6, 28).unwrap());
        assert_eq!(grid[3], Date::new(2026, 7, 1).unwrap());
        assert_eq!(grid[41], Date::new(2026, 8, 8).unwrap());
        for cell in &grid {
            assert_eq!(
                cell.weekday().index(),
                (cell.to_days() + 4).rem_euclid(7) as u32
            );
        }

        let monday = month_grid(2026, 7, Weekday::Monday);
        assert_eq!(monday[0].weekday(), Weekday::Monday);
        assert_eq!(monday[2], Date::new(2026, 7, 1).unwrap());
    }

    #[test]
    fn grid_when_month_starts_on_week_start() {
        // March 2026 starts on a Sunday: no leading cells.
        let grid = month_grid(2026, 3, Weekday::Sunday);
        assert_eq!(grid[0], Date::new(2026, 3, 1).unwrap());
    }

    #[test]
    fn formatting() {
        let d = Date::new(2026, 7, 4).unwrap();
        assert_eq!(d.format("YYYY-MM-DD"), "2026-07-04");
        assert_eq!(d.format("M/D/YYYY"), "7/4/2026");
        assert_eq!(d.format("MMM D, YYYY"), "Jul 4, 2026");
        assert_eq!(d.format("MMMM D"), "July 4");
        assert_eq!(d.to_string(), "2026-07-04");
    }

    #[test]
    fn iso_parsing() {
        assert_eq!(Date::parse_iso("2026-07-04"), Date::new(2026, 7, 4));
        assert_eq!(Date::parse_iso(" 2026-7-4 "), Date::new(2026, 7, 4));
        assert_eq!(Date::parse_iso("2026-02-30"), None);
        assert_eq!(Date::parse_iso("2026-07"), None);
        assert_eq!(Date::parse_iso("2026-07-04-01"), None);
        assert_eq!(Date::parse_iso("garbage"), None);
    }

    #[test]
    fn weekday_helpers() {
        assert_eq!(Weekday::Sunday.short(), "Su");
        assert_eq!(Weekday::from_index(8), Weekday::Monday);
        assert_eq!(Weekday::Saturday.index(), 6);
    }
}
