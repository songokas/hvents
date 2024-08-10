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
pub mod period;
pub mod print;
pub mod time;

use command::CommandEvent;
use data::Data;
use indexmap::{IndexMap, IndexSet};
use period::PeriodEvent;
use print::PrintEvent;
use serde::{de, Deserialize, Serialize};
use std::{borrow::Borrow, hash::Hash, path::PathBuf};
use time::{str_to_time, TimeResult};

use api_listen::ApiListenEvent;
use file_changed::FileChangedEvent;
use file_read::FileReadEvent;
use file_watch::WatchEvent;
use file_write::FileWriteEvent;
use mqtt_publish::MqttPublishEvent;
use mqtt_subscribe::MqttSubscribeEvent;

use self::{api_call::ApiCallEvent, time::TimeEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    MqttPublish(MqttPublishEvent),
    MqttSubscribe(MqttSubscribeEvent),
    #[serde(deserialize_with = "deserialize_time_event")]
    Time(TimeEvent),
    #[serde(deserialize_with = "deserialize_time_event")]
    Repeat(TimeEvent),
    Period(PeriodEvent),
    ApiCall(ApiCallEvent),
    ApiListen(ApiListenEvent),
    #[serde(deserialize_with = "deserialize_file_read_event")]
    FileRead(FileReadEvent),
    #[serde(deserialize_with = "deserialize_file_write_event")]
    FileWrite(FileWriteEvent),
    Watch(WatchEvent),
    FileChanged(FileChangedEvent),
    Execute(CommandEvent),
    Print(PrintEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencingEvent {
    #[serde(default)]
    pub name: EventName,
    #[serde(flatten)]
    pub event_type: EventType,
    pub next_event: Option<EventName>,
    pub next_event_template: Option<String>,
    #[serde(default)]
    pub data: Data,
    #[serde(default)]
    pub ignore_data: bool,
}

impl ReferencingEvent {
    pub fn merge(&mut self, data: Data) {
        if !self.ignore_data {
            self.data.merge(data)
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
            event.next_event = event.next_event.map(|name| format!("{prefix}_{name}"));
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
        OnlyTime(TimeResult),
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
