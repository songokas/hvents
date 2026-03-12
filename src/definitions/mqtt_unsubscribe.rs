use crate::{
    events::{EventType, ReferencingEvent, mqtt_unsubscribe::MqttUnsubscribeEvent},
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct MqttUnsubscribeDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(MqttUnsubscribeDefinition);
impl_create_event!(MqttUnsubscribeDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    Ok(ReferencingEvent {
        event_type: EventType::MqttUnsubscribe(MqttUnsubscribeEvent {
            topic: c.get_required("topic")?,
            pool_id: c.get("pool_id").unwrap_or_default(),
        }),
        ..Default::default()
    })
}
