//! Time filtering for file analysis.
//!
//! This module provides functionality to filter files based on their modification time.

use std::time::{Duration, SystemTime};

/// A filter for file modification times.
#[derive(Debug, Clone, Default)]
pub struct TimeFilter {
    /// Only include files modified after this time
    pub since: Option<SystemTime>,
    /// Only include files modified before this time
    pub until: Option<SystemTime>,
    /// Only include files older than this duration
    pub max_age: Option<Duration>,
    /// Only include files newer than this duration
    pub min_age: Option<Duration>,
}

impl TimeFilter {
    /// Create a new time filter with no restrictions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the minimum modification time (files must be newer than this).
    pub fn with_since(mut self, time: SystemTime) -> Self {
        self.since = Some(time);
        self
    }

    /// Set the maximum modification time (files must be older than this).
    pub fn with_until(mut self, time: SystemTime) -> Self {
        self.until = Some(time);
        self
    }

    /// Set the maximum age (files must be older than this duration).
    pub fn with_max_age(mut self, duration: Duration) -> Self {
        self.max_age = Some(duration);
        self
    }

    /// Set the minimum age (files must be newer than this duration).
    pub fn with_min_age(mut self, duration: Duration) -> Self {
        self.min_age = Some(duration);
        self
    }

    /// Check if a file with the given modification time should be included.
    pub fn should_include(&self, mtime: SystemTime) -> bool {
        let now = SystemTime::now();

        // Check since (file must be newer than this time)
        if let Some(since) = self.since {
            if mtime < since {
                return false;
            }
        }

        // Check until (file must be older than this time)
        if let Some(until) = self.until {
            if mtime > until {
                return false;
            }
        }

        // Check max_age (file must be older than this duration from now)
        if let Some(max_age) = self.max_age {
            if let Ok(age) = now.duration_since(mtime) {
                if age > max_age {
                    return false;
                }
            }
        }

        // Check min_age (file must be newer than this duration from now)
        if let Some(min_age) = self.min_age {
            if let Ok(age) = now.duration_since(mtime) {
                if age < min_age {
                    return false;
                }
            }
        }

        true
    }

    /// Check if the filter has any active restrictions.
    pub fn is_active(&self) -> bool {
        self.since.is_some()
            || self.until.is_some()
            || self.max_age.is_some()
            || self.min_age.is_some()
    }
}

/// Parse a duration string like "30d", "1w", "2h30m", etc.
///
/// Supported units:
/// - s: seconds
/// - m: minutes
/// - h: hours
/// - d: days
/// - w: weeks
/// - M: months (30 days)
/// - y: years (365 days)
pub fn parse_duration(s: &str) -> Result<Duration, ParseDurationError> {
    let mut total_secs: u64 = 0;
    let mut current_num = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() {
            current_num.push(c);
        } else {
            let num: u64 = current_num.parse().map_err(|_| ParseDurationError {
                input: s.to_string(),
                message: "Invalid number".to_string(),
            })?;
            current_num.clear();

            let secs = match c {
                's' => num,
                'm' => num * 60,
                'h' => num * 3600,
                'd' => num * 86400,
                'w' => num * 604800,
                'M' => num * 2592000, // 30 days
                'y' => num * 31536000, // 365 days
                _ => {
                    return Err(ParseDurationError {
                        input: s.to_string(),
                        message: format!("Unknown unit: {}", c),
                    })
                }
            };
            total_secs += secs;
        }
    }

    if !current_num.is_empty() {
        return Err(ParseDurationError {
            input: s.to_string(),
            message: "Trailing number without unit".to_string(),
        });
    }

    Ok(Duration::from_secs(total_secs))
}

/// Error when parsing a duration string.
#[derive(Debug, Clone, thiserror::Error)]
#[error("Failed to parse duration '{input}': {message}")]
pub struct ParseDurationError {
    input: String,
    message: String,
}

/// Parse a date string into a SystemTime.
///
/// Supported formats:
/// - YYYY-MM-DD
/// - YYYY-MM-DDTHH:MM:SS
/// - RFC3339 format
pub fn parse_date(s: &str) -> Result<SystemTime, ParseDateError> {
    // Try parsing as ISO date only (YYYY-MM-DD)
    if s.len() == 10 && s.chars().nth(4) == Some('-') && s.chars().nth(7) == Some('-') {
        let year: i32 = s[0..4].parse().map_err(|_| ParseDateError(s.to_string()))?;
        let month: u32 = s[5..7].parse().map_err(|_| ParseDateError(s.to_string()))?;
        let day: u32 = s[8..10].parse().map_err(|_| ParseDateError(s.to_string()))?;

        // Convert to Unix timestamp (simplified, assuming UTC)
        let timestamp = date_to_timestamp(year, month, day)?;
        return Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp));
    }

    // Try parsing with chrono if available
    #[cfg(feature = "chrono")]
    {
        use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};

        // Try RFC3339
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Ok(SystemTime::UNIX_EPOCH + Duration::from_secs(dt.timestamp() as u64));
        }

        // Try YYYY-MM-DDTHH:MM:SS
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            return Ok(SystemTime::UNIX_EPOCH
                + Duration::from_secs(dt.and_utc().timestamp() as u64));
        }
    }

    Err(ParseDateError(s.to_string()))
}

/// Simple date to Unix timestamp conversion (UTC, no leap seconds).
fn date_to_timestamp(year: i32, month: u32, day: u32) -> Result<u64, ParseDateError> {
    if month < 1 || month > 12 || day < 1 || day > 31 {
        return Err(ParseDateError(format!("Invalid date: {}-{}-{}", year, month, day)));
    }

    // Days per month (non-leap year)
    let days_in_month = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

    // Count days from 1970-01-01
    let mut days: i64 = 0;

    // Add days for complete years
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Add days for complete months in current year
    for m in 1..month {
        days += days_in_month[m as usize] as i64;
        if m == 2 && is_leap_year(year) {
            days += 1;
        }
    }

    // Add days in current month
    days += (day - 1) as i64;

    Ok((days * 86400) as u64)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Error when parsing a date string.
#[derive(Debug, Clone, thiserror::Error)]
#[error("Failed to parse date: {0}")]
pub struct ParseDateError(String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_filter_new() {
        let filter = TimeFilter::new();
        assert!(!filter.is_active());
    }

    #[test]
    fn test_time_filter_is_active() {
        let filter = TimeFilter::new().with_since(SystemTime::now());
        assert!(filter.is_active());

        let filter = TimeFilter::new().with_max_age(Duration::from_secs(3600));
        assert!(filter.is_active());
    }

    #[test]
    fn test_time_filter_should_include_since() {
        let now = SystemTime::now();
        let one_hour_ago = now - Duration::from_secs(3600);
        let two_hours_ago = now - Duration::from_secs(7200);

        let filter = TimeFilter::new().with_since(one_hour_ago);

        // File modified 30 minutes ago should be included
        assert!(filter.should_include(now - Duration::from_secs(1800)));

        // File modified 2 hours ago should be excluded
        assert!(!filter.should_include(two_hours_ago));
    }

    #[test]
    fn test_time_filter_should_include_until() {
        let now = SystemTime::now();
        let one_hour_ago = now - Duration::from_secs(3600);

        let filter = TimeFilter::new().with_until(one_hour_ago);

        // File modified 2 hours ago should be included
        assert!(filter.should_include(now - Duration::from_secs(7200)));

        // File modified 30 minutes ago should be excluded
        assert!(!filter.should_include(now - Duration::from_secs(1800)));
    }

    #[test]
    fn test_time_filter_max_age() {
        let filter = TimeFilter::new().with_max_age(Duration::from_secs(3600));

        let now = SystemTime::now();

        // File modified 30 minutes ago (age = 30 min < 1 hour)
        assert!(filter.should_include(now - Duration::from_secs(1800)));

        // File modified 2 hours ago (age = 2 hours > 1 hour)
        assert!(!filter.should_include(now - Duration::from_secs(7200)));
    }

    #[test]
    fn test_time_filter_min_age() {
        let filter = TimeFilter::new().with_min_age(Duration::from_secs(3600));

        let now = SystemTime::now();

        // File modified 2 hours ago (age = 2 hours > 1 hour)
        assert!(filter.should_include(now - Duration::from_secs(7200)));

        // File modified 30 minutes ago (age = 30 min < 1 hour)
        assert!(!filter.should_include(now - Duration::from_secs(1800)));
    }

    #[test]
    fn test_parse_duration_seconds() {
        let duration = parse_duration("30s").unwrap();
        assert_eq!(duration, Duration::from_secs(30));
    }

    #[test]
    fn test_parse_duration_minutes() {
        let duration = parse_duration("5m").unwrap();
        assert_eq!(duration, Duration::from_secs(300));
    }

    #[test]
    fn test_parse_duration_hours() {
        let duration = parse_duration("2h").unwrap();
        assert_eq!(duration, Duration::from_secs(7200));
    }

    #[test]
    fn test_parse_duration_days() {
        let duration = parse_duration("7d").unwrap();
        assert_eq!(duration, Duration::from_secs(604800));
    }

    #[test]
    fn test_parse_duration_weeks() {
        let duration = parse_duration("1w").unwrap();
        assert_eq!(duration, Duration::from_secs(604800));
    }

    #[test]
    fn test_parse_duration_months() {
        let duration = parse_duration("1M").unwrap();
        assert_eq!(duration, Duration::from_secs(2592000));
    }

    #[test]
    fn test_parse_duration_years() {
        let duration = parse_duration("1y").unwrap();
        assert_eq!(duration, Duration::from_secs(31536000));
    }

    #[test]
    fn test_parse_duration_combined() {
        let duration = parse_duration("1h30m").unwrap();
        assert_eq!(duration, Duration::from_secs(5400)); // 90 minutes
    }

    #[test]
    fn test_parse_duration_invalid() {
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("30").is_err()); // No unit
    }

    #[test]
    fn test_parse_date_iso() {
        let time = parse_date("2021-01-01").unwrap();
        let epoch = SystemTime::UNIX_EPOCH;
        let duration = time.duration_since(epoch).unwrap();
        // 2021-01-01 00:00:00 UTC = 1609459200
        assert_eq!(duration.as_secs(), 1609459200);
    }

    #[test]
    fn test_parse_date_invalid() {
        assert!(parse_date("not-a-date").is_err());
        assert!(parse_date("2021-13-01").is_err()); // Invalid month
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2020));
        assert!(!is_leap_year(2021));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
    }

    #[test]
    fn test_time_filter_combined() {
        let now = SystemTime::now();
        let two_hours_ago = now - Duration::from_secs(7200);
        let one_hour_ago = now - Duration::from_secs(3600);

        // Filter: files between 1 and 2 hours old
        let filter = TimeFilter::new()
            .with_since(two_hours_ago)
            .with_until(one_hour_ago);

        // File modified 1.5 hours ago should be included
        assert!(filter.should_include(now - Duration::from_secs(5400)));

        // File modified 30 minutes ago should be excluded (too new)
        assert!(!filter.should_include(now - Duration::from_secs(1800)));

        // File modified 3 hours ago should be excluded (too old)
        assert!(!filter.should_include(now - Duration::from_secs(10800)));
    }
}