use std::collections::HashMap;

use crate::{
    events::{EventType, ReferencingEvent, api_call::ApiCallEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct ApiCallDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(ApiCallDefinition);
impl_create_event!(ApiCallDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::ApiCall(ApiCallEvent {
            url: c.get_required_as("url")?,
            headers: c.get("headers").map(parse_headers).unwrap_or_default(),
            method: c.get_as("method")?.unwrap_or_default(),
            request_body: c.get("request_body"),
            request_content: c.get_as("request_content")?.unwrap_or_default(),
            response_content: c.get_as("response_content")?.unwrap_or_default(),
            pool_id: Default::default(),
        }),
        ..Default::default()
    })
}

fn parse_headers(headers: String) -> HashMap<String, String> {
    headers
        .split(',')
        .map_while(|s| {
            s.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect()
}
