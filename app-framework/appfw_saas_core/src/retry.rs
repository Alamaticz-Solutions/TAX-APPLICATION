use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackoffPolicy {
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
    pub jitter_ratio: f64,
}

impl Default for BackoffPolicy {
    fn default() -> Self {
        Self {
            initial_delay_ms: 250,
            max_delay_ms: 30_000,
            multiplier: 2.0,
            jitter_ratio: 0.2,
        }
    }
}

impl BackoffPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.initial_delay_ms == 0 {
            return Err("initial_delay_ms must be greater than 0".to_string());
        }
        if self.max_delay_ms < self.initial_delay_ms {
            return Err(
                "max_delay_ms must be greater than or equal to initial_delay_ms".to_string(),
            );
        }
        if !self.multiplier.is_finite() || self.multiplier < 1.0 {
            return Err("multiplier must be finite and greater than or equal to 1.0".to_string());
        }
        if !self.jitter_ratio.is_finite() || !(0.0..=1.0).contains(&self.jitter_ratio) {
            return Err("jitter_ratio must be finite and between 0.0 and 1.0".to_string());
        }
        Ok(())
    }

    pub fn delay_ms_for_attempt(&self, attempt: u32) -> u64 {
        if attempt <= 1 {
            return 0;
        }

        let exponent = (attempt - 2) as i32;
        let delay = (self.initial_delay_ms as f64) * self.multiplier.powi(exponent);
        delay.min(self.max_delay_ms as f64).round() as u64
    }

    pub fn jitter_bounds_ms(&self, delay_ms: u64) -> (u64, u64) {
        let jitter = (delay_ms as f64 * self.jitter_ratio).round() as u64;
        (
            delay_ms.saturating_sub(jitter),
            delay_ms.saturating_add(jitter).min(self.max_delay_ms),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RetryDecision {
    pub should_retry: bool,
    pub delay_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    #[serde(default)]
    pub retry_statuses: Vec<u16>,
    pub backoff: BackoffPolicy,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            retry_statuses: vec![408, 429, 500, 502, 503, 504],
            backoff: BackoffPolicy::default(),
        }
    }
}

impl RetryPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_attempts == 0 {
            return Err("max_attempts must be greater than 0".to_string());
        }

        for status in &self.retry_statuses {
            if *status < 100 || *status > 599 {
                return Err(format!(
                    "retry_statuses contains invalid HTTP status {status}"
                ));
            }
        }

        self.backoff.validate()
    }

    pub fn should_retry_status(&self, status: u16) -> bool {
        self.retry_statuses.contains(&status)
    }

    pub fn decision_for_status(&self, attempt: u32, status: u16) -> RetryDecision {
        let should_retry = attempt < self.max_attempts && self.should_retry_status(status);
        RetryDecision {
            should_retry,
            delay_ms: if should_retry {
                self.backoff.delay_ms_for_attempt(attempt + 1)
            } else {
                0
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_policy_calculates_capped_exponential_delays() {
        let policy = BackoffPolicy {
            initial_delay_ms: 100,
            max_delay_ms: 500,
            multiplier: 2.0,
            jitter_ratio: 0.1,
        };

        assert_eq!(policy.delay_ms_for_attempt(1), 0);
        assert_eq!(policy.delay_ms_for_attempt(2), 100);
        assert_eq!(policy.delay_ms_for_attempt(3), 200);
        assert_eq!(policy.delay_ms_for_attempt(4), 400);
        assert_eq!(policy.delay_ms_for_attempt(5), 500);
        assert_eq!(policy.jitter_bounds_ms(200), (180, 220));
    }

    #[test]
    fn retry_policy_retries_retryable_status_until_attempt_budget() {
        let policy = RetryPolicy::default();

        assert_eq!(
            policy.decision_for_status(1, 503),
            RetryDecision {
                should_retry: true,
                delay_ms: 250,
            }
        );
        assert_eq!(
            policy.decision_for_status(3, 503),
            RetryDecision {
                should_retry: false,
                delay_ms: 0,
            }
        );
        assert!(!policy.decision_for_status(1, 400).should_retry);
    }

    #[test]
    fn invalid_retry_policies_report_field_level_messages() {
        let policy = RetryPolicy {
            max_attempts: 0,
            ..RetryPolicy::default()
        };
        assert!(policy.validate().unwrap_err().contains("max_attempts"));

        let policy = RetryPolicy {
            retry_statuses: vec![99],
            ..RetryPolicy::default()
        };
        assert!(policy.validate().unwrap_err().contains("retry_statuses"));
    }
}
