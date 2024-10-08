use serde::{Deserialize, Serialize};

use crate::config::PoolId;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MqttPublishEvent {
    pub topic: String,
    pub body: Option<String>,
    #[serde(default)]
    pub retain: bool,
    #[serde(default)]
    pub pool_id: PoolId,
}
