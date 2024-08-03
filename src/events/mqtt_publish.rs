use serde::{Deserialize, Serialize};

use crate::config::PoolId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttPublishEvent {
    pub topic: String,
    pub template: Option<String>,
    #[serde(default)]
    pub retain: bool,
    #[serde(default)]
    pub pool_id: PoolId,
}
