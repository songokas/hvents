use serde::{Deserialize, Serialize};

use crate::config::PoolId;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MqttUnsubscribeEvent {
    pub topic: String,
    #[serde(default)]
    pub pool_id: PoolId,
}
