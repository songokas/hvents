use crate::{
    events::{EventType, ReferencingEvent, mqtt_publish::MqttPublishEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct MqttPublishDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(MqttPublishDefinition);
impl_create_event!(MqttPublishDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::MqttPublish(MqttPublishEvent {
            topic: c.get_required("topic")?,
            body: c.get("body"),
            retain: c.get("retain").is_some(),
            pool_id: c.get("pool_id").unwrap_or_default(),
        }),
        ..Default::default()
    })
}
