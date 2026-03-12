use crate::{
    events::{
        EventType, ReferencingEvent,
        comparison::{ComparisonEvent, DataToCompare},
    },
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct DataComparisonDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(DataComparisonDefinition);
impl_create_event!(DataComparisonDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    let data_to_compare: DataToCompare = c.get_required_as("data_type")?;
    Ok(ReferencingEvent {
        event_type: EventType::Comparison(ComparisonEvent {
            data_to_compare,
            key: c.get("key").unwrap_or_default(),
            operation: c.get_required_as("operation")?,
            value: c.get_required("value")?,
        }),
        ..Default::default()
    })
}
