use std::time::Duration;

use indexmap::IndexMap;
use log::info;
use rumqttc::{Client, Connection, MqttOptions};

use crate::config::{MqttConfiguration, PoolId};

#[derive(Default)]
pub struct MqttPool {
    clients: IndexMap<PoolId, Client>,
}

impl MqttPool {
    pub fn configure(&mut self, pool_id: PoolId, config: MqttConfiguration) -> Connection {
        let mut mqtt_options = MqttOptions::new(
            config.client_id.as_ref().unwrap_or(&pool_id),
            &config.host,
            config.port,
        );
        if let Some(user) = config.user {
            if let Some(pass) = config.pass {
                mqtt_options.set_credentials(user, pass);
            }
        }

        mqtt_options.set_keep_alive(Duration::from_secs(5));

        let (client, connection) = Client::new(mqtt_options, 10);

        info!("Connected to {}", config.host);

        self.clients.insert(pool_id, client);
        connection
    }

    pub fn get(&self, pool_id: &str) -> Option<&Client> {
        // return the first configuration when pool id is empty
        if pool_id.is_empty() {
            return self.clients.values().next();
        }
        self.clients.get(pool_id)
    }
}
