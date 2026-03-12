use crate::{
    events::{EventType, ReferencingEvent, print::PrintEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct PrintDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(PrintDefinition);
impl_create_event!(PrintDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::Print(PrintEvent {
            template: c.get_required("template")?,
            output: c.get_required_as("output")?,
        }),
        ..Default::default()
    })
}
