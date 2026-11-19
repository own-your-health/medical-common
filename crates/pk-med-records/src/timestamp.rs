//! A point in time read from RFC 3339 text (a FHIR `instant`, a stream's
//! `pulled_at`), comparable whatever offset or fractional digits each was
//! written with. Two such texts do not compare as strings:
//! `2026-02-28T19:00:00-05:00` and `2026-03-01T00:00:00Z` are one instant.

/// Seconds and nanoseconds since the Unix epoch, UTC. Ordered in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    secs: i64,
    nanos: u32,
}

impl Timestamp {
    /// `YYYY-MM-DDThh:mm:ss[.fraction](Z|±hh:mm)`, as RFC 3339 and FHIR's
    /// `instant` write it (`t` and `z` accepted too; digits past the ninth
    /// fractional one are dropped; a leap second `60` counts as the next
    /// one). `None` for anything else, a date alone included, and for a day
    /// the month does not have.
    pub fn parse(text: &str) -> Option<Timestamp> {
        let b = text.as_bytes();
        if b.len() < 20 {
            return None;
        }
        let year = digits(b, 0, 4)?;
        let month = digits(b, 5, 2)?;
        let day = digits(b, 8, 2)?;
        let hour = digits(b, 11, 2)?;
        let minute = digits(b, 14, 2)?;
        let second = digits(b, 17, 2)?;
        if b[4] != b'-'
            || b[7] != b'-'
            || !matches!(b[10], b'T' | b't')
            || b[13] != b':'
            || b[16] != b':'
        {
            return None;
        }
        if !(1..=12).contains(&month)
            || day < 1
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
            || second > 60
        {
            return None;
        }
        let mut i = 19;
        let mut nanos: u32 = 0;
        if b[i] == b'.' {
            i += 1;
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                if i - start < 9 {
                    nanos = nanos * 10 + u32::from(b[i] - b'0');
                }
                i += 1;
            }
            let used = i - start;
            if used == 0 {
                return None;
            }
            for _ in used..9 {
                nanos *= 10;
            }
        }
        let offset: i64 = match b.get(i..)? {
            [b'Z' | b'z'] => 0,
            [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2] => {
                let hours = digits(&[*h1, *h2], 0, 2)?;
                let minutes = digits(&[*m1, *m2], 0, 2)?;
                if hours > 23 || minutes > 59 {
                    return None;
                }
                let secs = i64::from(hours * 3600 + minutes * 60);
                if *sign == b'+' {
                    secs
                } else {
                    -secs
                }
            }
            _ => return None,
        };
        let days = days_from_civil(i64::from(year), month, day);
        let secs =
            days * 86_400 + i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second)
                - offset;
        Some(Timestamp { secs, nanos })
    }

    /// Whole seconds since the Unix epoch.
    pub fn unix_seconds(&self) -> i64 {
        self.secs
    }
}

/// `len` ASCII digits at `at`, as a number.
fn digits(b: &[u8], at: usize, len: usize) -> Option<u32> {
    let slice = b.get(at..at + len)?;
    slice.iter().try_fold(0u32, |n, &c| {
        c.is_ascii_digit().then(|| n * 10 + u32::from(c - b'0'))
    })
}

fn is_leap(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to a proleptic Gregorian date (Hinnant's
/// `days_from_civil`).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests;
