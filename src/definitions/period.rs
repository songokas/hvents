use crate::{
    events::{
        EventType, ReferencingEvent,
        period::{ExecutionPeriod, PeriodEvent},
    },
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct PeriodDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(PeriodDefinition);
impl_create_event!(PeriodDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::Period(PeriodEvent::new(ExecutionPeriod {
            from: c.get_required_as("from")?,
            to: c.get_required_as("to")?,
        })),
        ..Default::default()
    })
}
