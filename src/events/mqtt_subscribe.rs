use core::str::from_utf8;

use rust_fuzzy_search::{fuzzy_compare, fuzzy_search_threshold};
use serde::{Deserialize, Serialize};

use crate::config::PoolId;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MqttSubscribeEvent {
    pub topic: String,
    #[serde(flatten)]
    pub body: Option<MqttBodyMatch>,
    #[serde(default)]
    pub pool_id: PoolId,
    #[serde(default = "fuzzy_threshold_default")]
    pub fuzzy_threshold: f32,
}

impl MqttSubscribeEvent {
    pub fn matches(&self, topic: &str, body: &[u8]) -> bool {
        let topic_matches = if self.topic.ends_with('#') {
            topic.starts_with(self.topic.trim_end_matches('#'))
        } else if self.topic.contains("+") {
            self.topic
                .split('/')
                .zip(topic.split('/'))
                .all(|(expected, received)| expected == "+" || expected == received)
        } else {
            topic == self.topic
        };
        topic_matches
            && self
                .body
                .as_ref()
                .map(|b| b.matches(body, self.fuzzy_threshold))
                .unwrap_or(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MqttBodyMatch {
    Body(String),
    BodyMatchesAny(Vec<String>),
    BodyContains(String),
    BodyContainsAny(Vec<String>),
    FuzzyMatches(String),
    FuzzyMatchesAny(Vec<String>),
}

impl MqttBodyMatch {
    fn matches(&self, body: &[u8], fuzzy_threshold: f32) -> bool {
        match self {
            Self::Body(b) => Ok(b.as_str()) == from_utf8(body),
            Self::BodyMatchesAny(arr) => from_utf8(body)
                .map(|r| arr.iter().any(|s| r == s))
                .unwrap_or_default(),
            Self::BodyContains(b) => from_utf8(body).map(|r| r.contains(b)).unwrap_or_default(),
            Self::BodyContainsAny(arr) => from_utf8(body)
                .map(|r| arr.iter().any(|s| r.contains(s)))
                .unwrap_or_default(),
            Self::FuzzyMatches(b) => from_utf8(body)
                .map(|r| fuzzy_compare(r, b) > fuzzy_threshold)
                .unwrap_or_default(),
            Self::FuzzyMatchesAny(b) => from_utf8(body)
                .map(|r| {
                    !fuzzy_search_threshold(
                        r,
                        &b.iter().map(String::as_str).collect::<Vec<_>>(),
                        fuzzy_threshold,
                    )
                    .is_empty()
                })
                .unwrap_or_default(),
        }
    }
}

pub fn fuzzy_threshold_default() -> f32 {
    0.75f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches() {
        let data = [
            (
                "topic1",
                "payload".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic1".to_string(),
                    body: MqttBodyMatch::Body("payload".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                true,
            ),
            (
                "topic1",
                "payload".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic1".to_string(),
                    body: MqttBodyMatch::BodyMatchesAny(vec!["payload".to_string()]).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                true,
            ),
            (
                "topic1",
                "payload with data".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic1".to_string(),
                    body: MqttBodyMatch::BodyMatchesAny(vec!["payload".to_string()]).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                false,
            ),
            (
                "topic2",
                "payload with data".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic2".to_string(),
                    body: MqttBodyMatch::BodyContains("payload".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                true,
            ),
            (
                "topic3/hello",
                "payload with data".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic3/#".to_string(),
                    body: MqttBodyMatch::BodyContains("payload".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                true,
            ),
            (
                "unknown/hello",
                "payload with data".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic4/#".to_string(),
                    body: MqttBodyMatch::BodyContains("payload".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                false,
            ),
            (
                "topic5/hello",
                "just data".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic5/#".to_string(),
                    body: MqttBodyMatch::BodyContains("payload".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                false,
            ),
            (
                "topic5/hello",
                "payload with data".as_bytes(),
                MqttSubscribeEvent {
                    topic: "#".to_string(),
                    body: MqttBodyMatch::BodyContains("payload".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                true,
            ),
            (
                "topic5/hello",
                "payload with data".as_bytes(),
                MqttSubscribeEvent {
                    topic: "#".to_string(),
                    body: MqttBodyMatch::BodyContainsAny(vec!["payload".to_string()]).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                true,
            ),
            (
                "topic5/hello",
                "payload with data".as_bytes(),
                MqttSubscribeEvent {
                    topic: "#".to_string(),
                    body: MqttBodyMatch::BodyContainsAny(vec!["none".to_string()]).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                false,
            ),
            (
                "topic1/subject/hello/peter",
                "payload".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic1/+/hello/+".to_string(),
                    body: MqttBodyMatch::Body("payload".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                true,
            ),
            (
                "topic1/subject/hello/peter",
                "payload".as_bytes(),
                MqttSubscribeEvent {
                    topic: "+/hello".to_string(),
                    body: MqttBodyMatch::Body("payload".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                false,
            ),
            (
                "topic1/subject/hello/peter",
                "payload".as_bytes(),
                MqttSubscribeEvent {
                    topic: "+/+/hello/peter".to_string(),
                    body: MqttBodyMatch::Body("payload".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                true,
            ),
            (
                "topic1/subject/hello/peter",
                "payload".as_bytes(),
                MqttSubscribeEvent {
                    topic: "+/+/hello/peter".to_string(),
                    body: None,
                    pool_id: Default::default(),
                    fuzzy_threshold: 0f32,
                },
                true,
            ),
            (
                "fuzzy-match",
                "me testing you".as_bytes(),
                MqttSubscribeEvent {
                    topic: "fuzzy-match".to_string(),
                    body: MqttBodyMatch::FuzzyMatches("test".to_string()).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0.15f32,
                },
                true,
            ),
            (
                "fuzzy-match",
                "testing scenario".as_bytes(),
                MqttSubscribeEvent {
                    topic: "fuzzy-match".to_string(),
                    body: MqttBodyMatch::FuzzyMatchesAny(vec!["test".to_string()]).into(),
                    pool_id: Default::default(),
                    fuzzy_threshold: 0.15f32,
                },
                true,
            ),
        ];
        for (topic, body, event, equal) in data {
            assert_eq!(event.matches(topic, body), equal, "{topic}");
        }
    }
}
