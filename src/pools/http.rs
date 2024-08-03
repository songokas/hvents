use indexmap::IndexMap;

use crate::config::PoolId;
use crate::events::api_listen::HttpQueue;
use anyhow::Result;

#[derive(Default)]
pub struct HttpQueuePool {
    map: IndexMap<PoolId, HttpQueue>,
}

impl HttpQueuePool {
    pub fn configure(&mut self, pool_id: PoolId, queue: HttpQueue) -> Result<()> {
        self.map.insert(pool_id, queue);
        Ok(())
    }

    pub fn get(&self, pool_id: &str) -> Option<&HttpQueue> {
        // return the first configuration when the pool id is empty
        if pool_id.is_empty() {
            return self.map.values().next();
        }
        self.map.get(pool_id)
    }
}
