use crate::{
    events::{EventType, ReferencingEvent, scan_code_read::ScanCodeReadEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct ScanCodeDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(ScanCodeDefinition);
impl_create_event!(ScanCodeDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::ScanCodeRead(ScanCodeReadEvent::new(
            c.get_required_as("scan_code")?,
        )),
        ..Default::default()
    })
}
