use core::str::from_utf8;

use serde::{Deserialize, Serialize};

use crate::config::PoolId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttSubscribeEvent {
    pub topic: String,
    #[serde(flatten)]
    pub body: Option<MqttBodyMatch>,
    #[serde(default)]
    pub pool_id: PoolId,
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
        topic_matches && self.body.as_ref().map(|b| b.matches(body)).unwrap_or(true)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MqttBodyMatch {
    Body(String),
    BodyContains(String),
}

impl MqttBodyMatch {
    fn matches(&self, body: &[u8]) -> bool {
        match self {
            Self::Body(b) => Ok(b.as_str()) == from_utf8(body),
            Self::BodyContains(b) => from_utf8(body).map(|r| r.contains(b)).unwrap_or_default(),
        }
    }
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
                },
                true,
            ),
            (
                "topic2",
                "payload with data".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic2".to_string(),
                    body: MqttBodyMatch::BodyContains("payload".to_string()).into(),
                    pool_id: Default::default(),
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
                },
                true,
            ),
            (
                "topic1/subject/hello/peter",
                "payload".as_bytes(),
                MqttSubscribeEvent {
                    topic: "topic1/+/hello/+".to_string(),
                    body: MqttBodyMatch::Body("payload".to_string()).into(),
                    pool_id: Default::default(),
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
                },
                true,
            ),
        ];
        for (topic, body, event, equal) in data {
            assert_eq!(event.matches(topic, body), equal, "{topic}");
        }
    }
}
