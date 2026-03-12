use crate::{
    events::{EventType, ReferencingEvent, StateData},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct StateModificationDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(StateModificationDefinition);
impl_create_event!(StateModificationDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::Forward,
        state: StateData {
            count: None,
            replace: [(c.get_required("key")?, c.get_required("value")?)].into(),
        }
        .into(),
        ..Default::default()
    })
}
