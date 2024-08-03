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
pub mod time;

use command::CommandEvent;
use data::Data;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use std::{borrow::Borrow, hash::Hash};

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
    Time(TimeEvent),
    ApiCall(ApiCallEvent),
    ApiListen(ApiListenEvent),
    FileRead(FileReadEvent),
    FileWrite(FileWriteEvent),
    Watch(WatchEvent),
    FileChanged(FileChangedEvent),
    Execute(CommandEvent),
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
}

// impl ReferencingEvent {
//     pub fn merge_data(&mut self, b: Data) {
//         match self.data {
//             Data::Json(v) => merge_json_value(v, b)
//             Data::String(_) => todo!(),
//             Data::Bytes(_) => todo!(),
//             Data::Empty => todo!(),
//         }

//         // merge_json_value(&mut self.data, b);
//     }
// }

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
}

pub type EventName = String;
pub type EventMap = IndexMap<EventName, ReferencingEvent>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerMessage {
    pub executing_event: ExecutingEvent,
    pub event_to_execute: ReferencingEvent,
}

impl Eq for TimerMessage {}

impl PartialEq for TimerMessage {
    fn eq(&self, other: &Self) -> bool {
        self.executing_event.event_id == other.executing_event.event_id
    }
}

impl Hash for TimerMessage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.executing_event.event_id.hash(state);
    }
}

impl Borrow<str> for TimerMessage {
    fn borrow(&self) -> &str {
        &self.executing_event.event_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutingEvent {
    pub event_id: EventName,
    pub time_event: TimeEvent,
    pub name: EventName,
    pub next_event: Option<EventName>,
    pub next_event_template: Option<String>,
    pub data: Data,
}
