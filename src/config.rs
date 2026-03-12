use std::{collections::HashMap, path::PathBuf, sync::OnceLock};

use chrono::{Local, NaiveDateTime};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::events::{EventMap, EventName};

pub type ClientId = String;
pub type PoolId = String;
pub type Headers = HashMap<String, String>;
pub type QueueState = IndexMap<String, String>;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub start_with: Vec<EventName>,
    #[serde(default)]
    pub groups: IndexMap<String, PathBuf>,
    #[serde(default)]
    pub event_files: Vec<PathBuf>,
    #[serde(default)]
    pub events: EventMap,
    /// restore events from uri specified
    pub restore: Option<String>,
    pub location: Option<Location>,
    #[serde(default)]
    pub mqtt: IndexMap<PoolId, MqttConfiguration>,
    #[serde(default)]
    pub http: IndexMap<PoolId, String>,
    #[serde(default)]
    pub api: IndexMap<PoolId, ClientConfiguration>,
    /// pool id is currently not used for devices
    #[serde(default)]
    pub devices: IndexMap<PoolId, PathBuf>,
    #[serde(default)]
    pub metrics: Option<MetricsConfig>,
    #[serde(default)]
    pub initial_state: QueueState,
    pub definition_path: Option<PathBuf>,
}

#[derive(Deserialize)]
pub struct MetricsConfig {
    pub instance_id: String,
    #[serde(default = "service_name")]
    pub service_name: String,
    #[serde(default = "service_version")]
    pub service_version: String,
}

fn service_name() -> String {
    env!("CARGO_PKG_NAME").to_string()
}

fn service_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize)]
pub struct MqttConfiguration {
    pub host: String,
    pub user: Option<String>,
    pub pass: Option<String>,
    #[serde(default = "default_port")]
    pub port: u16,
    /// client id used for mqtt if it exists
    #[serde(default)]
    pub client_id: Option<ClientId>,
}

#[derive(Deserialize)]
pub struct ClientConfiguration {
    pub default_headers: Headers,
}

#[derive(Clone, Deserialize)]
pub struct DefinitionEvent {
    #[serde(default)]
    pub arguments: HashMap<String, String>,
    pub fields: HashMap<String, String>,
    pub expressions: Vec<String>,
    #[serde(default)]
    pub capture_group_sizes: HashMap<String, u8>,
}

#[derive(Deserialize)]
pub struct DefinitionConfig {
    capture_group_sizes: HashMap<String, u8>,
    events: IndexMap<String, DefinitionEvent>,
}

impl DefinitionConfig {
    pub fn get(&self, key: &str) -> Option<DefinitionEvent> {
        let mut event = self.events.get(key).cloned()?;
        event
            .capture_group_sizes
            .extend(self.capture_group_sizes.clone());
        Some(event)
    }
}

pub fn location() -> Option<(f64, f64)> {
    LOCATION.get().copied()
}

pub fn init_location(lat: f64, long: f64) {
    LOCATION.get_or_init(|| (lat, long));
}

pub fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

static LOCATION: OnceLock<(f64, f64)> = OnceLock::new();

fn default_port() -> u16 {
    1883
}

#[cfg(test)]
thread_local!(pub static INSTA_OUTPUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
