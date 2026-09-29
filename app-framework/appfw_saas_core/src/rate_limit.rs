use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApiUsageLimit {
    pub used: u64,
    pub limit: u64,
}

impl ApiUsageLimit {
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.used)
    }

    pub fn usage_ratio(&self) -> Option<f64> {
        (self.limit > 0).then_some(self.used as f64 / self.limit as f64)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct RateLimitSignal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_usage: Option<ApiUsageLimit>,
}

impl RateLimitSignal {
    pub fn from_headers<I, K, V>(headers: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut signal = Self::default();

        for (name, value) in headers {
            let normalized = name.as_ref().trim().to_ascii_lowercase();
            match normalized.as_str() {
                "retry-after" => {
                    signal.retry_after_ms = parse_retry_after_seconds(value.as_ref());
                }
                "sforce-limit-info" => {
                    signal.api_usage = parse_salesforce_limit_info(value.as_ref());
                }
                _ => {}
            }
        }

        signal
    }

    pub fn is_limited(&self) -> bool {
        self.retry_after_ms.is_some()
            || self
                .api_usage
                .as_ref()
                .is_some_and(|usage| usage.remaining() == 0)
    }
}

pub fn parse_retry_after_seconds(value: &str) -> Option<u64> {
    let value = value.trim();

    if let Ok(seconds) = value.parse::<u64>() {
        return Some(seconds.saturating_mul(1_000));
    }

    let retry_at = parse_imf_fixdate_epoch_seconds(value)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
        .unwrap_or(0);

    Some(retry_at.saturating_sub(now).saturating_mul(1_000))
}

pub fn parse_salesforce_limit_info(value: &str) -> Option<ApiUsageLimit> {
    value.split(',').find_map(|segment| {
        let (key, usage) = segment.trim().split_once('=')?;
        if key.trim() != "api-usage" {
            return None;
        }

        let (used, limit) = usage.trim().split_once('/')?;
        Some(ApiUsageLimit {
            used: used.trim().parse().ok()?,
            limit: limit.trim().parse().ok()?,
        })
    })
}

fn parse_imf_fixdate_epoch_seconds(value: &str) -> Option<u64> {
    let parts = value.split_ascii_whitespace().collect::<Vec<_>>();
    if parts.len() != 6 || !parts[0].ends_with(',') || parts[5] != "GMT" {
        return None;
    }

    let day = parts[1].parse::<u32>().ok()?;
    let month = match parts[2] {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year = parts[3].parse::<i32>().ok()?;
    let mut time_parts = parts[4].split(':');
    let hour = time_parts.next()?.parse::<u32>().ok()?;
    let minute = time_parts.next()?.parse::<u32>().ok()?;
    let second = time_parts.next()?.parse::<u32>().ok()?;
    if time_parts.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return None;
    }

    let days = days_since_unix_epoch(year, month, day)?;
    Some(
        days.saturating_mul(86_400)
            .saturating_add((hour as u64).saturating_mul(3_600))
            .saturating_add((minute as u64).saturating_mul(60))
            .saturating_add(second as u64),
    )
}

fn days_since_unix_epoch(year: i32, month: u32, day: u32) -> Option<u64> {
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }

    let adjusted_year = year - i32::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let month_prime = month as i32 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;

    Some(days.max(0) as u64)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_seconds_parses_to_milliseconds() {
        assert_eq!(parse_retry_after_seconds("10"), Some(10_000));
        assert_eq!(
            parse_retry_after_seconds(" Wed, 21 Oct 2015 07:28:00 GMT "),
            Some(0)
        );
        assert_eq!(
            parse_retry_after_seconds("Wed, 32 Oct 2015 07:28:00 GMT"),
            None
        );
    }

    #[test]
    fn salesforce_limit_info_extracts_api_usage_segment() {
        let usage =
            parse_salesforce_limit_info("api-usage=18/500000").expect("api usage should parse");

        assert_eq!(usage.used, 18);
        assert_eq!(usage.limit, 500_000);
        assert_eq!(usage.remaining(), 499_982);
        assert_eq!(usage.usage_ratio(), Some(0.000036));
    }

    #[test]
    fn salesforce_limit_info_ignores_unrelated_segments() {
        let usage = parse_salesforce_limit_info("per-app-api-usage=3/100, api-usage=99/100")
            .expect("api usage should parse");

        assert_eq!(usage.used, 99);
        assert_eq!(usage.remaining(), 1);
        assert_eq!(parse_salesforce_limit_info("per-app-api-usage=3/100"), None);
    }

    #[test]
    fn rate_limit_signal_collects_known_headers_case_insensitively() {
        let signal = RateLimitSignal::from_headers([
            ("Retry-After", "5"),
            ("Sforce-Limit-Info", "api-usage=100/100"),
            ("Content-Type", "application/json"),
        ]);

        assert_eq!(signal.retry_after_ms, Some(5_000));
        assert_eq!(signal.api_usage.as_ref().unwrap().remaining(), 0);
        assert!(signal.is_limited());
    }
}
