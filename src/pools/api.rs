use indexmap::IndexMap;
use reqwest::blocking::Client;

use crate::config::{ClientConfiguration, PoolId};
use anyhow::anyhow;
use anyhow::Result;

#[derive(Default)]
pub struct ClientPool {
    clients: IndexMap<PoolId, Client>,
}

impl ClientPool {
    pub fn configure(&mut self, pool_id: PoolId, config: &ClientConfiguration) -> Result<()> {
        let headers = (&config.default_headers)
            .try_into()
            .map_err(|e| anyhow!("Failed to set default headers {e}"))?;
        let client = Client::builder().default_headers(headers).build()?;
        self.clients.insert(pool_id, client);
        Ok(())
    }

    pub fn get(&self, pool_id: &str) -> Option<&Client> {
        // return the first configuration when the pool id is empty
        if pool_id.is_empty() {
            return self.clients.values().next();
        }
        self.clients.get(pool_id)
    }
}
