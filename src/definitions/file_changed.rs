use crate::{
    events::{EventType, ReferencingEvent, file_changed::FileChangedEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct FileChangedDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(FileChangedDefinition);
impl_create_event!(FileChangedDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::FileChanged(FileChangedEvent {
            path: c.get_required_as("file_path")?,
            when: c.get_as("when")?.unwrap_or_default(),
        }),
        ..Default::default()
    })
}
