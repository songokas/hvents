use anyhow::Result;

use crate::events::ReferencingEvent;

pub trait EventFactory {
    fn create_event(&self, input: String) -> Result<Option<ReferencingEvent>>;
}
