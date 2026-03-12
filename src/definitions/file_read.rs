use crate::{
    events::{EventType, ReferencingEvent, file_read::FileReadEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct FileReadDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(FileReadDefinition);
impl_create_event!(FileReadDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::FileRead(FileReadEvent {
            file: c.get_required_as("file_path")?,
            data_type: c.get_as("data_type")?.unwrap_or_default(),
        }),
        ..Default::default()
    })
}
