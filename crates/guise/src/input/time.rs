//! `Time` — a wall-clock time of day plus parsing/formatting for the pickers.

use std::fmt;

/// Hour/minute time of day (24-hour internally).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time {
    hour: u32,
    minute: u32,
}

impl Time {
    /// A validated time, or `None` when out of range.
    pub fn new(hour: u32, minute: u32) -> Option<Time> {
        if hour < 24 && minute < 60 {
            Some(Time { hour, minute })
        } else {
            None
        }
    }

    pub fn hour(self) -> u32 {
        self.hour
    }

    pub fn minute(self) -> u32 {
        self.minute
    }

    /// Hour on a 12-hour clock (12, 1..=11) and whether it is PM.
    pub fn hour_12(self) -> (u32, bool) {
        let pm = self.hour >= 12;
        let hour = match self.hour % 12 {
            0 => 12,
            h => h,
        };
        (hour, pm)
    }

    /// The same time with a different hour/meridiem, e.g. from picker columns.
    pub fn with_hour_12(self, hour_12: u32, pm: bool) -> Time {
        let base = hour_12 % 12;
        Time {
            hour: if pm { base + 12 } else { base },
            minute: self.minute,
        }
    }

    pub fn with_hour(self, hour: u32) -> Time {
        Time {
            hour: hour.min(23),
            ..self
        }
    }

    pub fn with_minute(self, minute: u32) -> Time {
        Time {
            minute: minute.min(59),
            ..self
        }
    }

    /// "14:05".
    pub fn format_24(self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// "2:05 PM".
    pub fn format_12(self) -> String {
        let (hour, pm) = self.hour_12();
        format!(
            "{}:{:02} {}",
            hour,
            self.minute,
            if pm { "PM" } else { "AM" }
        )
    }

    /// Parse "14:05", "2:05 PM", "2:05pm", "02:05 am".
    pub fn parse(s: &str) -> Option<Time> {
        let s = s.trim();
        let lower = s.to_ascii_lowercase();
        let (clock, meridiem) = if let Some(rest) = lower.strip_suffix("pm") {
            (rest.trim_end(), Some(true))
        } else if let Some(rest) = lower.strip_suffix("am") {
            (rest.trim_end(), Some(false))
        } else {
            (lower.as_str(), None)
        };
        let (h, m) = clock.split_once(':')?;
        let hour: u32 = h.trim().parse().ok()?;
        let minute: u32 = m.trim().parse().ok()?;
        match meridiem {
            None => Time::new(hour, minute),
            Some(pm) => {
                if hour == 0 || hour > 12 {
                    return None;
                }
                let base = hour % 12;
                Time::new(if pm { base + 12 } else { base }, minute)
            }
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format_24())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation() {
        assert!(Time::new(23, 59).is_some());
        assert!(Time::new(24, 0).is_none());
        assert!(Time::new(0, 60).is_none());
    }

    #[test]
    fn twelve_hour_conversion() {
        assert_eq!(Time::new(0, 30).unwrap().hour_12(), (12, false));
        assert_eq!(Time::new(12, 0).unwrap().hour_12(), (12, true));
        assert_eq!(Time::new(13, 15).unwrap().hour_12(), (1, true));
        assert_eq!(Time::new(11, 59).unwrap().hour_12(), (11, false));
    }

    #[test]
    fn with_hour_12_round_trips() {
        for hour in 0..24 {
            let t = Time::new(hour, 42).unwrap();
            let (h12, pm) = t.hour_12();
            assert_eq!(t.with_hour_12(h12, pm), t);
        }
    }

    #[test]
    fn formatting() {
        assert_eq!(Time::new(14, 5).unwrap().format_24(), "14:05");
        assert_eq!(Time::new(14, 5).unwrap().format_12(), "2:05 PM");
        assert_eq!(Time::new(0, 0).unwrap().format_12(), "12:00 AM");
        assert_eq!(Time::new(12, 0).unwrap().format_12(), "12:00 PM");
        assert_eq!(Time::new(9, 30).unwrap().to_string(), "09:30");
    }

    #[test]
    fn parsing() {
        assert_eq!(Time::parse("14:05"), Time::new(14, 5));
        assert_eq!(Time::parse("2:05 PM"), Time::new(14, 5));
        assert_eq!(Time::parse("2:05pm"), Time::new(14, 5));
        assert_eq!(Time::parse("12:00 am"), Time::new(0, 0));
        assert_eq!(Time::parse("12:00 PM"), Time::new(12, 0));
        assert_eq!(Time::parse(" 09:30 "), Time::new(9, 30));
        assert_eq!(Time::parse("25:00"), None);
        assert_eq!(Time::parse("13:00 PM"), None);
        assert_eq!(Time::parse("0:30 am"), None);
        assert_eq!(Time::parse("nope"), None);
    }
}
