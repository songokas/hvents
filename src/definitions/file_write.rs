use crate::{
    events::{EventType, ReferencingEvent, file_write::FileWriteEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct FileWriteDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(FileWriteDefinition);
impl_create_event!(FileWriteDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::FileWrite(FileWriteEvent {
            file: c.get_required_as("file_path")?,
            mode: c.get_as("mode")?.unwrap_or_default(),
        }),
        ..Default::default()
    })
}
