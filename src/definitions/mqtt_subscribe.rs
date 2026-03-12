use crate::{
    events::{
        EventType, ReferencingEvent,
        mqtt_subscribe::{MqttBodyMatch, MqttSubscribeEvent, fuzzy_threshold_default},
    },
    gherkin::{
        expression_parser::FieldExpression,
        field_captures::FieldCaptures,
        macros::{impl_create_event, impl_from_config},
    },
};

pub struct MqttSubscribeDefinition {
    expressions: Vec<FieldExpression>,
}

impl_from_config!(MqttSubscribeDefinition);
impl_create_event!(MqttSubscribeDefinition);

fn create_event(c: FieldCaptures) -> Result<ReferencingEvent, anyhow::Error> {
    let body_type = c.get("body_type");
    let body = match body_type.as_deref() {
        Some("matches") => MqttBodyMatch::Body(c.get_required("body")?).into(),
        Some("matches any") => MqttBodyMatch::BodyMatchesAny(
            c.get_required("body")?
                .split(',')
                .map(ToOwned::to_owned)
                .collect(),
        )
        .into(),
        Some("contains") => MqttBodyMatch::BodyContains(c.get_required("body")?).into(),
        Some("contains any") => MqttBodyMatch::BodyContainsAny(
            c.get_required("body")?
                .split(',')
                .map(ToOwned::to_owned)
                .collect(),
        )
        .into(),
        Some("fuzzy matches") => MqttBodyMatch::FuzzyMatches(c.get_required("body")?).into(),
        Some("fuzzy matches any") => MqttBodyMatch::FuzzyMatchesAny(
            c.get_required("body")?
                .split(',')
                .map(ToOwned::to_owned)
                .collect(),
        )
        .into(),
        Some(t) => panic!("Unknown type {t}"),
        _ => c.get("body").map(MqttBodyMatch::Body),
    };
    Ok(ReferencingEvent {
        event_type: EventType::MqttSubscribe(MqttSubscribeEvent {
            topic: c.get_required("topic")?,
            body,
            pool_id: c.get("pool_id").unwrap_or_default(),
            fuzzy_threshold: c
                .get_as("fuzzy_threshold")?
                .unwrap_or_else(|| fuzzy_threshold_default()),
        }),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use crate::{config::DefinitionConfig, gherkin::event_factory::EventFactory};

    use super::*;

    #[test]
    fn test_create_event() {
        let config: DefinitionConfig =
            serde_yaml::from_str(include_str!("definitions.yaml")).unwrap();
        let d =
            MqttSubscribeDefinition::from_config(config.get("mqtt_subscribe").unwrap()).unwrap();
        let event = d
            .create_event(
                r#"mqtt message from topic "test/topic" and body matches any "simple,body""#
                    .to_string(),
            )
            .unwrap()
            .unwrap();
        assert!(matches!(event.event_type, EventType::MqttSubscribe(_)));
    }
}
