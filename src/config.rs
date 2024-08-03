use std::{collections::HashMap, path::PathBuf, sync::OnceLock};

use chrono::{DateTime, Local};
use indexmap::IndexMap;
use serde::Deserialize;

use crate::events::{EventMap, EventName};

pub type ClientId = String;
pub type PoolId = String;
pub type Headers = HashMap<String, String>;

#[derive(Deserialize)]
pub struct Config {
    pub start_with: Vec<EventName>,
    #[serde(default)]
    pub groups: IndexMap<String, PathBuf>,
    #[serde(default)]
    pub event_files: Vec<PathBuf>,
    #[serde(default)]
    pub events: EventMap,
    pub restore: Option<String>,
    pub location: Option<Location>,
    #[serde(default)]
    pub mqtt: IndexMap<PoolId, MqttConfiguration>,
    #[serde(default)]
    pub http: IndexMap<PoolId, String>,
    #[serde(default)]
    pub api: IndexMap<PoolId, ClientConfiguration>,
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

pub fn location() -> Option<(f64, f64)> {
    LOCATION.get().copied()
}

pub fn init_location(lat: f64, long: f64) {
    LOCATION.get_or_init(|| (lat, long));
}

pub fn now() -> DateTime<Local> {
    Local::now()
}

static LOCATION: OnceLock<(f64, f64)> = OnceLock::new();

fn default_port() -> u16 {
    1883
}
