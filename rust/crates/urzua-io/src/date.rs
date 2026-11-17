use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

const SECS_PER_DAY: i64 = 86_400;

/// Years a Unix time can name: [`Date::from_unix`] answers `None` outside them.
/// Callers treat `None` as "no date", so the bounds are part of the contract.
const MIN_YEAR: i64 = -262_143;
const MAX_YEAR: i64 = 262_142;

/// A proleptic Gregorian calendar date. Fields run most to least significant,
/// so the derived order is chronological.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Date {
    year: i64,
    month: u8,
    day: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParseError {
    /// Not ten characters of `dddd-dd-dd`.
    Shape,
    /// Well shaped, but no such day (month 13, 30 February, ...).
    NotACalendarDate,
}

impl Date {
    pub(crate) fn from_ymd(year: i64, month: u8, day: u8) -> Option<Date> {
        let valid = (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month);
        valid.then_some(Date { year, month, day })
    }

    /// Exactly `YYYY-MM-DD`: four ASCII digits, hyphen, two, hyphen, two. No
    /// sign, padding, whitespace or trailing text is accepted.
    pub(crate) fn parse(value: &str) -> Result<Date, ParseError> {
        let b = value.as_bytes();
        let shaped = b.len() == 10
            && b.iter().enumerate().all(|(i, c)| match i {
                4 | 7 => *c == b'-',
                _ => c.is_ascii_digit(),
            });
        if !shaped {
            return Err(ParseError::Shape);
        }
        let num = |range: std::ops::Range<usize>| {
            b[range]
                .iter()
                .fold(0i64, |n, c| n * 10 + i64::from(c - b'0'))
        };
        Date::from_ymd(num(0..4), num(5..7) as u8, num(8..10) as u8)
            .ok_or(ParseError::NotACalendarDate)
    }

    pub(crate) fn from_unix(unix: i64) -> Option<Date> {
        let date = Date::from_days(unix.div_euclid(SECS_PER_DAY));
        (MIN_YEAR..=MAX_YEAR).contains(&date.year).then_some(date)
    }

    /// The wall clock as Unix seconds. Only `system_clock` calls it.
    pub(crate) fn unix_now() -> i64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(e) => {
                let d = e.duration();
                -(d.as_secs() as i64) - i64::from(d.subsec_nanos() > 0)
            }
        }
    }

    /// Unix seconds of the last second of this date, UTC.
    pub(crate) fn end_of_day_unix(self) -> i64 {
        (self.to_days() + 1) * SECS_PER_DAY - 1
    }

    /// Days since 1970-01-01; negative before it.
    pub(crate) fn to_days(self) -> i64 {
        // Shifting the year to start on 1 March puts the leap day last in its
        // year, so the day-of-year of a month is a linear function of the month.
        // 400-year eras of 146097 days repeat the calendar exactly; floor
        // division keeps years before year 0 in the right era.
        let m = i64::from(self.month);
        let y = self.year - i64::from(m <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Inverse of [`Date::to_days`]; the same era arithmetic run backwards.
    pub(crate) fn from_days(days: i64) -> Date {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let year = yoe + era * 400 + i64::from(month <= 2);
        Date { year, month, day }
    }
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: u8) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `YYYY-MM-DD`. A year outside 0000..=9999 keeps at least four digits and
/// carries a sign (`-0001`, `+10000`), so the text stays unambiguous.
impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.year {
            0..=9999 => write!(f, "{:04}", self.year)?,
            y if y < 0 => write!(f, "-{:04}", -y)?,
            y => write!(f, "+{y}")?,
        }
        write!(f, "-{:02}-{:02}", self.month, self.day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i64, m: u8, d: u8) -> Date {
        Date::from_ymd(y, m, d).unwrap()
    }

    #[test]
    fn days_and_civil_dates_round_trip_and_advance_one_day_at_a_time() {
        let first = ymd(1900, 1, 1).to_days();
        let last = ymd(2200, 12, 31).to_days();
        let mut prev = Date::from_days(first);
        assert_eq!(prev, ymd(1900, 1, 1));
        for days in first..=last {
            let date = Date::from_days(days);
            assert_eq!(date.to_days(), days, "{date}");
            assert_eq!(Date::parse(&date.to_string()), Ok(date), "{date}");
            if days > first {
                let next = if prev.day < days_in_month(prev.year, prev.month) {
                    ymd(prev.year, prev.month, prev.day + 1)
                } else if prev.month < 12 {
                    ymd(prev.year, prev.month + 1, 1)
                } else {
                    ymd(prev.year + 1, 1, 1)
                };
                assert_eq!(date, next, "after {prev}");
                assert!(date > prev);
            }
            prev = date;
        }
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        for (year, leap) in [
            (1900, false),
            (2000, true),
            (2100, false),
            (2024, true),
            (2025, false),
            (0, true),
        ] {
            assert_eq!(is_leap(year), leap, "{year}");
            assert_eq!(Date::from_ymd(year, 2, 29).is_some(), leap, "{year}");
        }
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2025, 2), 28);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn known_anchors() {
        assert_eq!(ymd(1970, 1, 1).to_days(), 0);
        assert_eq!(ymd(1969, 12, 31).to_days(), -1);
        assert_eq!(ymd(2000, 3, 1).to_days(), 11_017);
        assert_eq!(ymd(2024, 2, 29).to_days(), 19_782);
        assert_eq!(ymd(2026, 11, 17).to_days(), 20_774);
        assert_eq!(Date::from_days(-1), ymd(1969, 12, 31));
        assert_eq!(Date::from_days(11_017), ymd(2000, 3, 1));
        assert_eq!(Date::from_days(19_782), ymd(2024, 2, 29));
        assert_eq!(Date::from_days(20_774), ymd(2026, 11, 17));
        assert_eq!(ymd(0, 1, 1).to_days(), -719_528);
    }

    #[test]
    fn dates_order_and_compare_by_day_count() {
        assert!(ymd(2026, 11, 17) < ymd(2026, 12, 1));
        assert!(ymd(2025, 12, 31) < ymd(2026, 1, 1));
        assert!(ymd(1999, 1, 1) < ymd(2000, 1, 1));
        assert_eq!(ymd(2026, 11, 17).to_days() - ymd(2026, 11, 10).to_days(), 7);
    }

    #[test]
    fn display_is_the_iso_date() {
        assert_eq!(ymd(2026, 1, 5).to_string(), "2026-01-05");
        assert_eq!(ymd(7, 3, 9).to_string(), "0007-03-09");
        assert_eq!(ymd(0, 1, 1).to_string(), "0000-01-01");
        assert_eq!(ymd(-1, 12, 31).to_string(), "-0001-12-31");
        assert_eq!(ymd(10_000, 1, 1).to_string(), "+10000-01-01");
        assert_eq!(ymd(-262_143, 1, 1).to_string(), "-262143-01-01");
    }

    #[test]
    fn unix_times_before_the_epoch_and_out_of_range() {
        assert_eq!(Date::from_unix(0), Some(ymd(1970, 1, 1)));
        assert_eq!(Date::from_unix(-1), Some(ymd(1969, 12, 31)));
        assert_eq!(Date::from_unix(86_399), Some(ymd(1970, 1, 1)));
        assert_eq!(Date::from_unix(86_400), Some(ymd(1970, 1, 2)));
        assert_eq!(Date::from_unix(-86_400), Some(ymd(1969, 12, 31)));
        assert_eq!(Date::from_unix(-86_401), Some(ymd(1969, 12, 30)));
        assert_eq!(Date::from_unix(i64::MAX), None);
        assert_eq!(Date::from_unix(i64::MIN), None);
        let (lo, hi) = (ymd(MIN_YEAR, 1, 1), ymd(MAX_YEAR, 12, 31));
        assert_eq!(Date::from_unix(lo.to_days() * SECS_PER_DAY), Some(lo));
        assert_eq!(Date::from_unix(lo.to_days() * SECS_PER_DAY - 1), None);
        assert_eq!(Date::from_unix(hi.end_of_day_unix()), Some(hi));
        assert_eq!(Date::from_unix(hi.end_of_day_unix() + 1), None);
    }

    #[test]
    fn end_of_day_is_the_last_second_of_the_date() {
        assert_eq!(ymd(1970, 1, 1).end_of_day_unix(), 86_399);
        assert_eq!(ymd(1969, 12, 31).end_of_day_unix(), -1);
        assert_eq!(ymd(2026, 11, 8).end_of_day_unix(), 1_794_182_399);
    }

    #[test]
    fn today_is_a_valid_date_string() {
        let unix = Date::unix_now();
        let text = Date::from_unix(unix).unwrap().to_string();
        let parsed = Date::parse(&text).unwrap();
        assert_eq!(parsed.to_string(), text);
        assert!(parsed >= ymd(2026, 1, 1));
        assert!(Date::unix_now() >= unix);
    }
}
