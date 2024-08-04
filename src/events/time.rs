use core::{fmt::Display, str::FromStr};
use std::time::Duration;

use chrono::{DateTime, Datelike, Days, Local, NaiveDateTime, NaiveTime};
use human_date_parser::{from_human_time, ParseError, ParseResult};
use serde::{de, Deserialize, Serialize};

use crate::config::{location, now};

pub const COOL_DOWN_DURATION: Duration = Duration::from_millis(3000);
pub const EXECUTION_PERIOD: Duration = Duration::from_millis(1000);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimeEvent {
    pub execute_period: Option<ExecutionPeriod>,
    #[serde(default)]
    #[serde(deserialize_with = "str_to_time_optional")]
    pub execute_time: Option<TimeResult>,
}

impl TimeEvent {
    pub fn matches(&self, now: DateTime<Local>) -> bool {
        match &self.execute_time {
            Some(t) => t.within_execution_period(now),
            None => true,
        }
    }

    pub fn can_execute(&self, now: DateTime<Local>) -> bool {
        match &self.execute_period {
            Some(t) => t.matches(now),
            None => true,
        }
    }

    pub fn expired(&self, now: DateTime<Local>) -> bool {
        match &self.execute_time {
            Some(TimeResult::Time(_)) => false,
            Some(t) => t.lt(now - EXECUTION_PERIOD),
            None => true,
        }
    }

    pub fn reset(mut self) -> Self {
        if let Some(p) = self.execute_period {
            self.execute_period = ExecutionPeriod {
                from: p.from.reset(),
                to: p.to.reset(),
            }
            .into();
        }
        if let Some(p) = self.execute_time {
            self.execute_time = p.reset().into();
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeResult {
    // datetime and date can change depending on the supplied value
    DateTime((DateTime<Local>, String)),
    Date((NaiveDateTime, String)),
    Time((NaiveTime, String)),
}

impl TimeResult {
    pub fn gte(&self, now: DateTime<Local>) -> bool {
        match self {
            Self::DateTime((d, _)) => *d >= now,
            Self::Date((d, _)) => *d >= now.naive_local(),
            Self::Time((d, _)) => *d >= now.naive_local().time(),
        }
    }

    pub fn lte(&self, now: DateTime<Local>) -> bool {
        match self {
            Self::DateTime((d, _)) => *d <= now,
            Self::Date((d, _)) => *d <= now.naive_local(),
            Self::Time((d, _)) => *d <= now.naive_local().time(),
        }
    }

    pub fn within_execution_period(&self, now: DateTime<Local>) -> bool {
        match self {
            Self::DateTime((d, _)) => (now - *d)
                .abs()
                .to_std()
                .map(|s| s < EXECUTION_PERIOD)
                .unwrap_or_default(),
            Self::Date((d, _)) => (now.naive_local() - *d)
                .abs()
                .to_std()
                .map(|s| s < EXECUTION_PERIOD)
                .unwrap_or_default(),
            Self::Time((d, _)) => (now.naive_local().time() - *d)
                .abs()
                .to_std()
                .map(|s| s < EXECUTION_PERIOD)
                .unwrap_or_default(),
        }
    }

    pub fn gt(&self, now: DateTime<Local>) -> bool {
        match self {
            Self::DateTime((d, _)) => *d > now,
            Self::Date((d, _)) => *d > now.naive_local(),
            Self::Time((d, _)) => *d > now.naive_local().time(),
        }
    }

    pub fn lt(&self, now: DateTime<Local>) -> bool {
        match self {
            Self::DateTime((d, _)) => *d < now,
            Self::Date((d, _)) => *d < now.naive_local(),
            Self::Time((d, _)) => *d < now.naive_local().time(),
        }
    }

    pub fn reset(self) -> Self {
        let supplied_str = match self {
            Self::DateTime((_, s)) => s,
            Self::Date((_, s)) => s,
            Self::Time((_, s)) => s,
        };

        supplied_str.parse().expect("time can not change")
    }
}

impl FromStr for TimeResult {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid_value = || ParseError::ValueInvalid {
            amount: s.to_string(),
        };
        if s.contains("sunset") || s.contains("sunrise") {
            if let Some((lat, long)) = location() {
                return parse_sunrise_sunset(s, lat, long);
            } else {
                return Err(invalid_value());
            }
        }

        Ok(match from_human_time(s)? {
            ParseResult::Date(d) => {
                TimeResult::Date((NaiveDateTime::new(d, NaiveTime::default()), s.to_string()))
            }
            ParseResult::Time(d) => TimeResult::Time((d, s.to_string())),
            ParseResult::DateTime(d) => TimeResult::DateTime((d, s.to_string())),
        })
    }
}

fn parse_sunrise_sunset(s: &str, lat: f64, long: f64) -> Result<TimeResult, ParseError> {
    let invalid_value = || ParseError::ValueInvalid {
        amount: s.to_string(),
    };
    let replace_sunset = s.starts_with("sunset");
    let replace_sunrise = s.starts_with("sunrise");

    let result = if replace_sunset {
        let sunset = s.replace("sunset", "");
        let s = if sunset.trim().is_empty() {
            "now"
        } else {
            sunset.trim()
        };
        from_human_time(s)
    } else if replace_sunrise {
        let sunrise = s.replace("sunrise", "");
        let s = if sunrise.trim().is_empty() {
            "now"
        } else {
            sunrise.trim()
        };
        from_human_time(s)
    } else {
        from_human_time(s)
    };

    Ok(match result? {
        ParseResult::Date(d) => {
            let (sunrise, sunset) =
                sunrise::sunrise_sunset(lat, long, d.year(), d.month(), d.day());

            let dt: DateTime<Local> = if s.contains("sunrise") {
                DateTime::from_timestamp(sunrise, 0)
                    .map(Into::into)
                    .ok_or_else(invalid_value)?
            } else if s.contains("sunset") {
                DateTime::from_timestamp(sunset, 0)
                    .map(Into::into)
                    .ok_or_else(invalid_value)?
            } else {
                return Err(invalid_value());
            };
            TimeResult::Date((dt.naive_local(), s.to_string()))
        }
        ParseResult::Time(_) => return Err(invalid_value()),
        ParseResult::DateTime(d) => {
            let calculate = |d: DateTime<Local>| {
                let (sunrise, sunset) =
                    sunrise::sunrise_sunset(lat, long, d.year(), d.month(), d.day());
                if replace_sunrise {
                    Ok(DateTime::from_timestamp(sunrise, 0)
                        .ok_or_else(invalid_value)?
                        .into())
                } else if replace_sunset {
                    Ok(DateTime::from_timestamp(sunset, 0)
                        .ok_or_else(invalid_value)?
                        .into())
                } else {
                    Err(invalid_value())
                }
            };

            let sun_dt: DateTime<Local> = calculate(d)?;
            let now = now();
            let time_diff = now.naive_local().time() - d.naive_local().time();

            // if its today an sunrise/sunset happened calculate next
            let dt = if sun_dt.date_naive() == now.date_naive() && now >= sun_dt {
                calculate(
                    now.checked_add_days(Days::new(1))
                        .ok_or_else(invalid_value)?,
                )? - time_diff
            } else {
                sun_dt - time_diff
            };

            TimeResult::DateTime((dt, s.to_string()))
        }
    })
}

impl Display for TimeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DateTime((d, _)) => write!(f, "{}", d.naive_local()),
            Self::Date((d, _)) => write!(f, "{}", d),
            Self::Time((d, _)) => write!(f, "{}", d),
        }
    }
}

fn str_to_time<'de, D>(deserializer: D) -> Result<TimeResult, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum StringOrTime {
        String(String),
        Time(TimeResult),
    }
    let s: StringOrTime = de::Deserialize::deserialize(deserializer)?;
    match s {
        StringOrTime::String(s) => s.parse().map_err(de::Error::custom),
        StringOrTime::Time(t) => Ok(t),
    }
}

fn str_to_time_optional<'de, D>(deserializer: D) -> Result<Option<TimeResult>, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum StringOrTime {
        String(String),
        Time(TimeResult),
    }
    let s: Option<StringOrTime> = de::Deserialize::deserialize(deserializer)?;
    match s {
        Some(StringOrTime::String(s)) => s
            .parse::<TimeResult>()
            .map(Into::into)
            .map_err(de::Error::custom),
        Some(StringOrTime::Time(t)) => Ok(t.into()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Days, Duration, Local, NaiveDate, Timelike};

    use crate::config::{init_location, now};

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
            let time_event: TimeEvent = serde_json::from_str(&format!(
                r#"{{"execute_period":{{"from":"{from}", "to":"{to}"}}}}"#
            ))
            .unwrap();
            assert_eq!(
                time_event.can_execute(now),
                expected,
                "{from} {to} {time_event:?} {now}"
            );
        }
    }

    #[test]
    fn test_execution_time_from_json() {
        let data = [
            ("now", now(), true),
            (
                "today",
                now().with_time(NaiveTime::default()).unwrap(),
                true,
            ),
            (
                "22:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap())
                    .unwrap(),
                true,
            ),
            (
                "22:00:00",
                now()
                    .with_time(NaiveTime::from_hms_milli_opt(22, 0, 0, 999).unwrap())
                    .unwrap(),
                true,
            ),
            (
                "22:00:01",
                now()
                    .with_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap())
                    .unwrap(),
                false,
            ),
            (
                "22:00:00",
                now()
                    .with_time(NaiveTime::from_hms_milli_opt(21, 59, 59, 1).unwrap())
                    .unwrap(),
                true,
            ),
            (
                "21:59:59",
                now()
                    .with_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap())
                    .unwrap(),
                false,
            ),
        ];
        for (time, now, expected) in data {
            let time_event: TimeEvent =
                serde_json::from_str(&format!(r#"{{"execute_time":"{time}"}}"#)).unwrap();
            assert_eq!(
                time_event.matches(now),
                expected,
                "{time} {time_event:?} {now}"
            );
        }
    }

    #[test]
    fn test_execution_time_expired_from_json() {
        let data = [
            ("now", now(), false),
            ("yesterday 12:00", now(), true),
            (
                "today",
                now().with_time(NaiveTime::default()).unwrap(),
                false,
            ),
            (
                "yesterday",
                now().with_time(NaiveTime::default()).unwrap(),
                true,
            ),
            (
                "22:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap())
                    .unwrap(),
                false,
            ),
            // time only events do not expire
            (
                "21:00",
                now()
                    .with_time(NaiveTime::from_hms_opt(22, 0, 0).unwrap())
                    .unwrap(),
                false,
            ),
        ];
        for (time, now, expired) in data {
            let time_event: TimeEvent =
                serde_json::from_str(&format!(r#"{{"execute_time":"{time}"}}"#)).unwrap();
            assert_eq!(time_event.expired(now), expired, "{time} {time_event:?}");
        }
    }

    #[test]
    fn test_time_result_matches() {
        let now = now();
        let in_few_seconds = now + Duration::seconds(2);
        let time = TimeResult::Time((
            now.naive_local().time(),
            now.naive_local().time().to_string(),
        ));
        assert!(time.gte(now));
        assert!(time.within_execution_period(now));
        assert!(time.lt(in_few_seconds));
        assert!(!time.gte(in_few_seconds));
        assert!(!time.within_execution_period(in_few_seconds));
        assert!(!time.lt(now));

        let time = time.reset();
        //TODO milliseconds are not parsed
        assert!(!time.gte(now));
        assert!(time.within_execution_period(now));
        assert!(time.lt(in_few_seconds));
    }

    #[test]
    fn test_date_result_matches() {
        let now = now();
        let tomorrow = now
            .checked_add_days(Days::new(1))
            .unwrap()
            .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap();
        let time = TimeResult::Date((now.naive_local(), "tomorrow".to_string()));
        assert!(time.gte(now));
        assert!(time.within_execution_period(now));
        assert!(time.lt(now.checked_add_days(Days::new(1)).unwrap()));

        assert!(!time.gte(tomorrow));
        assert!(!time.within_execution_period(tomorrow));
        assert!(!time.lt(now));

        let time = time.reset();
        assert!(time.gte(tomorrow));
        assert!(time.within_execution_period(tomorrow));
        assert!(time.lt(tomorrow.checked_add_days(Days::new(1)).unwrap()));
    }

    #[test]
    fn test_date_time_result_matches() {
        let now = now();
        let tomorrow = Local::now()
            .checked_add_days(Days::new(1))
            .unwrap()
            .with_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap())
            .unwrap();
        let time = TimeResult::DateTime((now, "tomorrow 12:00".to_string()));

        assert!(time.gte(now));
        assert!(time.within_execution_period(now));
        assert!(time.lt(now.checked_add_days(Days::new(1)).unwrap()));

        assert!(!time.gte(tomorrow));
        assert!(!time.within_execution_period(tomorrow));
        assert!(!time.lt(now));

        let time = time.reset();
        assert!(time.gte(tomorrow));
        assert!(time.within_execution_period(tomorrow));
        assert!(time.lt(tomorrow.checked_add_days(Days::new(1)).unwrap()));
    }

    #[test]
    fn test_sunrise_from_str() {
        init_location(52.37403, 4.88969);
        let data = [
            (
                "2024-07-31 sunrise",
                NaiveDate::from_ymd_opt(2024, 7, 31)
                    .unwrap()
                    .and_hms_opt(6, 59, 37)
                    .unwrap()
                    .and_local_timezone(Local)
                    .unwrap()
                    .into(),
            ),
            (
                "2024-07-31 sunset",
                NaiveDate::from_ymd_opt(2024, 7, 31)
                    .unwrap()
                    .and_hms_opt(22, 33, 51)
                    .unwrap()
                    .and_local_timezone(Local)
                    .unwrap()
                    .into(),
            ),
            // disabled because now can not be changed in the library
            // (
            //     "sunset in 1 hour",
            //     NaiveDate::from_ymd_opt(2024, 7, 31)
            //         .unwrap()
            //         .and_hms_opt(3, 59, 37)
            //         .unwrap()
            //         .and_local_timezone(Local)
            //         .unwrap()
            //         .into(),
            // ),
            // (
            //     "sunset",
            //     NaiveDate::from_ymd_opt(2024, 7, 31)
            //         .unwrap()
            //         .and_hms_opt(19, 33, 51)
            //         .unwrap()
            //         .and_local_timezone(Local)
            //         .unwrap()
            //         .into(),
            // ),
            // (
            //     "sunset in 1 hours",
            //     NaiveDate::from_ymd_opt(2024, 7, 31)
            //         .unwrap()
            //         .and_hms_opt(20, 33, 51)
            //         .unwrap()
            //         .and_local_timezone(Local)
            //         .unwrap()
            //         .into(),
            // ),
            // (
            //     "sunrise 1 hours ago",
            //     NaiveDate::from_ymd_opt(2024, 7, 31)
            //         .unwrap()
            //         .and_hms_opt(2, 59, 37)
            //         .unwrap()
            //         .and_local_timezone(Local)
            //         .unwrap()
            //         .into(),
            // ),
        ];
        for (time, now) in data {
            let time_result = time.parse::<TimeResult>();
            if let Some(now) = now {
                let time_result = time_result.unwrap();
                assert!(
                    time_result.within_execution_period(now),
                    "{time} {time_result:?} {now}"
                );
            } else {
                assert!(time_result.is_err());
            }
        }
    }

    #[test]
    fn test_serialize_deserialize_time_event() {
        let now = now();
        let time = TimeEvent {
            execute_time: TimeResult::DateTime((now, "tomorrow 12:00".to_string())).into(),
            execute_period: None,
        };
        let s = serde_json::to_string(&time).unwrap();
        let result: TimeEvent = serde_json::from_str(&s).unwrap();
        assert!(result.matches(now));
    }

    #[test]
    #[ignore = "test weekday parsing"]
    fn test_relative_time() {
        let hour = now().hour();
        let data = [
            "monday".to_string(),
            "in 10s".to_string(),
            "wednesday 11:00".to_string(),
            "this week wednesday 11:00".to_string(),
            format!("{hour}:00"),
            "in 1 day".to_string(),
        ];
        for time in data {
            let time_event: TimeEvent =
                serde_json::from_str(&format!(r#"{{"execute_time":"{time}"}}"#)).unwrap();
            dbg!(&time_event);
        }
    }
}
