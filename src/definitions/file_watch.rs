use crate::{
    events::{EventType, ReferencingEvent, file_watch::WatchEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct FileWatchDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(FileWatchDefinition);
impl_create_event!(FileWatchDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::Watch(WatchEvent {
            path: c.get_required_as("file_path")?,
            action: c.get_as("action")?.unwrap_or_default(),
            recursive: c.get("resursive").is_some(),
        }),
        ..Default::default()
    })
}
