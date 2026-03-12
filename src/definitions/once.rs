use crate::{
    events::{EventType, ReferencingEvent, time::TimeEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct OnceDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(OnceDefinition);
impl_create_event!(OnceDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::Time(TimeEvent {
            execute_time: c.get_required_as("execute_time")?,
            event_id: c.get("event_id"),
        }),
        ..Default::default()
    })
}
