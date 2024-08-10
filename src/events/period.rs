use std::time::Duration;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::events::time::str_to_time;

use super::time::TimeResult;

pub const COOL_DOWN_DURATION: Duration = Duration::from_millis(3000);
pub const EXECUTION_PERIOD: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PeriodEvent(ExecutionPeriod);

impl PeriodEvent {
    pub fn new(period: ExecutionPeriod) -> Self {
        Self(period)
    }

    pub fn is_within_period(&self, now: DateTime<Local>) -> bool {
        self.0.matches(now)
    }

    pub fn reset(mut self) -> Self {
        self.0.from = self.0.from.reset();
        self.0.to = self.0.to.reset();
        self
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionPeriod {
    #[serde(deserialize_with = "str_to_time")]
    pub from: TimeResult,
    #[serde(deserialize_with = "str_to_time")]
    pub to: TimeResult,
}

impl ExecutionPeriod {
    pub fn matches(&self, now: DateTime<Local>) -> bool {
        // for time when its less than from
        if matches!((&self.from, &self.to), (TimeResult::Time(f), TimeResult::Time(t)) if f > t) {
            self.from.lte(now) || self.to.gt(now)
        } else {
            self.from.lte(now) && self.to.gt(now)
        }
    }
}

#[cfg(test)]
mod tests {

    use chrono::NaiveTime;

    use crate::config::now;

    use super::*;

    #[test]
    fn test_execution_within_period_from_json() {
        let data = [
            ("a second ago", "in 2 minutes", now(), true),
            ("a second ago", "in 2 hours", now(), true),
            ("today", "tomorrow", now(), true),
            (
                "22:00",
                "23:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap())
                    .unwrap(),
                true,
            ),
            (
                "22:00",
                "23:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(22, 59, 59).unwrap())
                    .unwrap(),
                true,
            ),
            (
                "22:00",
                "3:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap())
                    .unwrap(),
                true,
            ),
            (
                "22:00",
                "3:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(2, 59, 59).unwrap())
                    .unwrap(),
                true,
            ),
            (
                "22:00",
                "3:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(3, 0, 0).unwrap())
                    .unwrap(),
                false,
            ),
            (
                "22:00",
                "3:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(21, 59, 59).unwrap())
                    .unwrap(),
                false,
            ),
            (
                "22:00",
                "3:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(17, 0, 0).unwrap())
                    .unwrap(),
                false,
            ),
        ];
        for (from, to, now, expected) in data {
            let time_event: PeriodEvent =
                serde_json::from_str(&format!(r#"{{"from":"{from}", "to":"{to}"}}"#)).unwrap();
            assert_eq!(
                time_event.is_within_period(now),
                expected,
                "{from} {to} {time_event:?} {now}"
            );
        }
    }
}
