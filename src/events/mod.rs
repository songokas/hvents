pub mod api_call;
pub mod api_listen;
pub mod command;
pub mod data;
pub mod file_changed;
pub mod file_read;
pub mod file_watch;
pub mod file_write;
pub mod mqtt_publish;
pub mod mqtt_subscribe;
pub mod mqtt_unsubscribe;
pub mod period;
pub mod print;
pub mod time;

use command::CommandEvent;
use core::ops::Deref;
use data::{Data, Metadata};
use indexmap::{IndexMap, IndexSet};
use mqtt_unsubscribe::MqttUnsubscribeEvent;
use period::PeriodEvent;
use print::PrintEvent;
use serde::{de, Deserialize, Serialize};
use std::{borrow::Borrow, hash::Hash, path::PathBuf};
use time::{str_to_time, ExecuteTime};

use api_listen::ApiListenEvent;
use file_changed::FileChangedEvent;
use file_read::FileReadEvent;
use file_watch::WatchEvent;
use file_write::FileWriteEvent;
use mqtt_publish::MqttPublishEvent;
use mqtt_subscribe::MqttSubscribeEvent;

use self::{api_call::ApiCallEvent, time::TimeEvent};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    #[serde(deserialize_with = "deserialize_mqtt_publish_event")]
    MqttPublish(MqttPublishEvent),
    #[serde(deserialize_with = "deserialize_mqtt_subscribe_event")]
    MqttSubscribe(MqttSubscribeEvent),
    #[serde(deserialize_with = "deserialize_mqtt_unsubscribe_event")]
    MqttUnsubscribe(MqttUnsubscribeEvent),
    #[serde(deserialize_with = "deserialize_time_event")]
    Time(TimeEvent),
    #[serde(deserialize_with = "deserialize_time_event")]
    Repeat(TimeEvent),
    Period(PeriodEvent),
    #[serde(deserialize_with = "deserialize_api_call_event")]
    ApiCall(ApiCallEvent),
    #[serde(deserialize_with = "deserialize_api_listen_event")]
    ApiListen(ApiListenEvent),
    #[serde(deserialize_with = "deserialize_file_read_event")]
    FileRead(FileReadEvent),
    #[serde(deserialize_with = "deserialize_file_write_event")]
    FileWrite(FileWriteEvent),
    #[serde(deserialize_with = "deserialize_watch_event")]
    Watch(WatchEvent),
    #[serde(deserialize_with = "deserialize_file_changed_event")]
    FileChanged(FileChangedEvent),
    Execute(CommandEvent),
    Print(PrintEvent),
    #[default]
    Pass,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ReferencingEvent {
    #[serde(default)]
    pub name: EventName,
    #[serde(flatten)]
    #[serde(deserialize_with = "deserialize_event_type")]
    pub event_type: EventType,
    #[serde(flatten)]
    pub next_event: Option<NextEvent>,
    #[serde(default)]
    pub metadata: Metadata,
    pub state: Option<StateData>,
    #[serde(default)]
    pub data: Data,
    #[serde(default)]
    pub merge_data: MergePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateData {
    pub count: Option<String>,
    #[serde(default)]
    pub replace: IndexMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub enum NextEvent {
    #[serde(rename = "next_event")]
    Name(EventName),
    #[serde(rename = "next_event_template")]
    Template(String),
}

impl Deref for NextEvent {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            NextEvent::Name(s) => s.deref(),
            NextEvent::Template(_) => "unknown",
        }
    }
}

impl From<&'static str> for NextEvent {
    fn from(value: &'static str) -> Self {
        Self::Name(value.to_string())
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, Copy)]
#[serde(rename_all = "lowercase")]
pub enum MergePolicy {
    #[default]
    Yes,
    No,
    Overwrite,
}

impl ReferencingEvent {
    pub fn merge(&mut self, data: Data) {
        self.data.merge_with_policy(data, self.merge_data);
    }

    pub fn try_merge_bytes(&mut self, bytes: &[u8]) {
        match self.merge_data {
            MergePolicy::Yes => self.data.try_merge_bytes(bytes),
            MergePolicy::No => (),
            MergePolicy::Overwrite => self.data = Data::Bytes(bytes.to_vec()),
        }
    }

    pub fn event_id(&self) -> &str {
        if let EventType::Time(t) | EventType::Repeat(t) = &self.event_type {
            t.event_id.as_deref().unwrap_or(&self.name)
        } else {
            &self.name
        }
    }

    pub fn time_event(&self) -> Option<&TimeEvent> {
        if let EventType::Time(t) | EventType::Repeat(t) = &self.event_type {
            Some(t)
        } else {
            None
        }
    }
}

impl Eq for ReferencingEvent {}

impl PartialEq for ReferencingEvent {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for ReferencingEvent {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl Borrow<str> for ReferencingEvent {
    fn borrow(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Events(IndexSet<ReferencingEvent>);

impl Events {
    pub fn new(events: IndexSet<ReferencingEvent>) -> Self {
        Self(events)
    }

    pub fn get_event_by_name(&self, name: &str) -> Option<ReferencingEvent> {
        self.0.get(name).cloned()
    }

    pub fn get_next_event(&self, event: &ReferencingEvent) -> Option<ReferencingEvent> {
        // generate a new pass event since next event is unknown and only event executor
        // knows how to handle it
        match &event.next_event {
            Some(NextEvent::Template(s)) => ReferencingEvent {
                name: format!("generated_from_{}", event.name),
                next_event: NextEvent::Template(s.clone()).into(),
                ..Default::default()
            }
            .into(),
            Some(NextEvent::Name(s)) => self.0.get(s.as_str()).cloned(),
            None => None,
        }
    }

    pub fn get_event_id(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(|e| e.event_id())
    }

    pub fn has_event_by_name(&self, name: &str) -> bool {
        self.0.contains(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ReferencingEvent> {
        self.0.iter()
    }

    pub fn merge_with_prefix(mut self, events: EventMap, prefix: &str) -> Self {
        self.0.extend(events.into_iter().map(|(name, mut event)| {
            event.name = format!("{prefix}_{name}");
            if let Some(NextEvent::Name(name)) = event.next_event {
                event.next_event = NextEvent::Name(format!("{prefix}_{name}")).into()
            }
            event
        }));
        self
    }

    pub fn merge(mut self, events: EventMap) -> Self {
        self.0.extend(events.into_iter().map(|(name, mut event)| {
            event.name = name;
            event
        }));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub type EventName = String;
pub type EventMap = IndexMap<EventName, ReferencingEvent>;

fn deserialize_time_event<'de, D>(deserializer: D) -> Result<TimeEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum TimeOrFull {
        #[serde(deserialize_with = "str_to_time")]
        OnlyTime(ExecuteTime),
        Full(TimeEvent),
    }
    let s: TimeOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        TimeOrFull::OnlyTime(execute_time) => Ok(TimeEvent {
            execute_time,
            event_id: None,
        }),
        TimeOrFull::Full(t) => Ok(t),
    }
}

fn deserialize_file_read_event<'de, D>(deserializer: D) -> Result<FileReadEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OneOrFull {
        One(PathBuf),
        Full(FileReadEvent),
    }
    let s: OneOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        OneOrFull::One(file) => Ok(FileReadEvent {
            file,
            data_type: Default::default(),
        }),
        OneOrFull::Full(t) => Ok(t),
    }
}

fn deserialize_file_write_event<'de, D>(deserializer: D) -> Result<FileWriteEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OneOrFull {
        One(PathBuf),
        Full(FileWriteEvent),
    }
    let s: OneOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        OneOrFull::One(file) => Ok(FileWriteEvent {
            file,
            mode: Default::default(),
        }),
        OneOrFull::Full(t) => Ok(t),
    }
}

fn deserialize_event_type<'de, D>(deserializer: D) -> Result<EventType, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: Option<EventType> = de::Deserialize::deserialize(deserializer)?;
    Ok(match s {
        Some(e) => e,
        None => EventType::Pass,
    })
}

fn deserialize_mqtt_publish_event<'de, D>(deserializer: D) -> Result<MqttPublishEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OneOrFull {
        One(String),
        Full(MqttPublishEvent),
    }
    let s: OneOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        OneOrFull::One(topic) => Ok(MqttPublishEvent {
            topic,
            ..Default::default()
        }),
        OneOrFull::Full(t) => Ok(t),
    }
}

fn deserialize_mqtt_subscribe_event<'de, D>(deserializer: D) -> Result<MqttSubscribeEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OneOrFull {
        One(String),
        Full(MqttSubscribeEvent),
    }
    let s: OneOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        OneOrFull::One(topic) => Ok(MqttSubscribeEvent {
            topic,
            ..Default::default()
        }),
        OneOrFull::Full(t) => Ok(t),
    }
}

fn deserialize_mqtt_unsubscribe_event<'de, D>(
    deserializer: D,
) -> Result<MqttUnsubscribeEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OneOrFull {
        One(String),
        Full(MqttUnsubscribeEvent),
    }
    let s: OneOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        OneOrFull::One(topic) => Ok(MqttUnsubscribeEvent {
            topic,
            ..Default::default()
        }),
        OneOrFull::Full(t) => Ok(t),
    }
}

fn deserialize_watch_event<'de, D>(deserializer: D) -> Result<WatchEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OneOrFull {
        One(PathBuf),
        Full(WatchEvent),
    }
    let s: OneOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        OneOrFull::One(path) => Ok(WatchEvent {
            path,
            ..Default::default()
        }),
        OneOrFull::Full(t) => Ok(t),
    }
}

fn deserialize_file_changed_event<'de, D>(deserializer: D) -> Result<FileChangedEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OneOrFull {
        One(PathBuf),
        Full(FileChangedEvent),
    }
    let s: OneOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        OneOrFull::One(path) => Ok(FileChangedEvent {
            path,
            ..Default::default()
        }),
        OneOrFull::Full(t) => Ok(t),
    }
}

fn deserialize_api_listen_event<'de, D>(deserializer: D) -> Result<ApiListenEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OneOrFull {
        One(String),
        Full(ApiListenEvent),
    }
    let s: OneOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        OneOrFull::One(path) => Ok(ApiListenEvent {
            path,
            ..Default::default()
        }),
        OneOrFull::Full(t) => Ok(t),
    }
}

fn deserialize_api_call_event<'de, D>(deserializer: D) -> Result<ApiCallEvent, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum OneOrFull {
        One(String),
        Full(ApiCallEvent),
    }
    let s: OneOrFull = de::Deserialize::deserialize(deserializer)?;
    match s {
        OneOrFull::One(url) => Ok(ApiCallEvent {
            url,
            ..Default::default()
        }),
        OneOrFull::Full(t) => Ok(t),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_deserialize_no_event_type() {
        let expected = ReferencingEvent {
            name: "test1".to_string(),
            event_type: EventType::Pass,
            next_event: NextEvent::Name("test2".to_string()).into(),
            metadata: json!({"meta1":"metavalue1"}).into(),
            state: StateData {
                count: "count_key".to_string().into(),
                replace: IndexMap::new(),
            }
            .into(),
            data: Data::Json(json!({"data1": "value1"})),
            merge_data: MergePolicy::Overwrite,
        };
        let yaml = r#"
                name: test1
                next_event: test2
                metadata:
                    meta1: metavalue1
                state: 
                    count: count_key
                data:
                    data1: value1
                merge_data: overwrite
        "#;
        let event: ReferencingEvent = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(event, expected);
    }

    #[test]
    fn test_deserialize_mqtt_subscribe() {
        let expected = ReferencingEvent {
            name: "test1".to_string(),
            event_type: EventType::MqttSubscribe(MqttSubscribeEvent {
                topic: "topic".to_string(),
                body: None,
                pool_id: "".to_string(),
            }),
            next_event: None,
            metadata: json!({}).into(),
            state: StateData {
                count: None,
                replace: [("count_key".to_string(), "1".to_string())]
                    .into_iter()
                    .collect(),
            }
            .into(),
            data: Data::String("datavalue".to_string()),
            merge_data: MergePolicy::No,
        };
        let yaml = r#"
                name: test1
                mqtt_subscribe: topic
                state: 
                    replace: 
                        count_key: 1
                data: datavalue
                merge_data: no
        "#;
        let event: ReferencingEvent = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(event, expected);
    }
}
