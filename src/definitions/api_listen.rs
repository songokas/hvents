use crate::{
    events::{EventType, ReferencingEvent, api_listen::ApiListenEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct ApiListenDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(ApiListenDefinition);
impl_create_event!(ApiListenDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::ApiListen(ApiListenEvent {
            path: c.get_required_as("path")?,
            method: c.get_as("method")?.unwrap_or_default(),
            request_content: c.get_as("request_content")?.unwrap_or_default(),
            request_headers: Default::default(),
            response_headers: Default::default(),
            response_body: c.get("response_body"),
            response_content: c.get_as("response_content")?.unwrap_or_default(),
            action: Default::default(),
            pool_id: Default::default(),
        }),
        ..Default::default()
    })
}
