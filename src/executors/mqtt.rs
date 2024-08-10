use std::sync::mpsc::Sender;

use log::{debug, error};
use rumqttc::{Connection, Event, Incoming};

use crate::events::{EventType, Events, ReferencingEvent};

pub fn mqtt_executor(
    mut connection: Connection,
    events: &Events,
    queue_tx: Sender<ReferencingEvent>,
) -> Result<(), anyhow::Error> {
    let mut show_error = true;
    for notification in connection.iter() {
        match notification {
            Ok(Event::Incoming(Incoming::Publish(packet))) => {
                show_error = true;
                debug!("Incoming mqtt event {} {:?}", packet.topic, packet.payload);
                if let Some(e) = handle_incoming(events, &packet.topic, &packet.payload) {
                    queue_tx.send(e)?;
                }
            }
            Ok(_) => {
                show_error = true;
                continue;
            }
            Err(e) => {
                if show_error {
                    error!("Receive mqtt error {e}. Suppressing further messages until success");
                }
                show_error = false;
            }
        };
    }
    Ok(())
}

fn handle_incoming(events: &Events, topic: &str, payload: &[u8]) -> Option<ReferencingEvent> {
    let event_associated = events
        .iter()
        .find_map(|ref_event| match &ref_event.event_type {
            EventType::MqttSubscribe(e) if e.matches(topic, payload) => {
                debug!(
                    "Event found event {} next event {:?}",
                    ref_event.name, ref_event.next_event
                );
                ref_event.next_event.as_ref()
            }
            _ => None,
        })?;

    if let Some(mut event) = events.get_event_by_name(event_associated) {
        event.data.try_merge_bytes(payload);
        Some(event)
    } else {
        debug!("Received event without further handler {event_associated}");
        None
    }
}

#[cfg(test)]
mod tests {

    use crate::events::{
        mqtt_subscribe::{MqttBodyMatch, MqttSubscribeEvent},
        EventName,
    };

    use super::*;

    #[test]
    fn test_handle_incoming() {
        let events = Events::new(
            [
                create_mqtt_event(
                    "test1".to_string(),
                    Some("test2".to_string()),
                    "topic1",
                    MqttBodyMatch::Body("content1".to_string()),
                ),
                create_mqtt_event(
                    "test2".to_string(),
                    Some("expected".to_string()),
                    "topic2",
                    MqttBodyMatch::Body("content2".to_string()),
                ),
                create_mqtt_event(
                    "test3".to_string(),
                    Some("test2".to_string()),
                    "topic3",
                    MqttBodyMatch::ContainsString("content3".to_string()),
                ),
                create_mqtt_event(
                    "test4".to_string(),
                    Some("test2".to_string()),
                    "topic1",
                    MqttBodyMatch::ContainsString("content4".to_string()),
                ),
            ]
            .into_iter()
            .collect(),
        );
        let event = handle_incoming(&events, "topic1", b"content1");
        assert_eq!(event.unwrap().next_event.unwrap(), "expected");
        let event = handle_incoming(&events, "topic2", b"content2");
        // no referencing event
        assert!(event.is_none());
        let event = handle_incoming(&events, "topic3", b"content3");
        assert_eq!(event.unwrap().next_event.unwrap(), "expected");

        let event = handle_incoming(&events, "topic1", b"content4");
        assert_eq!(event.unwrap().next_event.unwrap(), "expected");
    }

    fn create_mqtt_event(
        name: String,
        event: Option<EventName>,
        topic: &str,
        body: MqttBodyMatch,
    ) -> ReferencingEvent {
        ReferencingEvent {
            name,
            event_type: EventType::MqttSubscribe(MqttSubscribeEvent {
                topic: topic.to_string(),
                body,
                pool_id: Default::default(),
            }),
            next_event: event,
            data: Default::default(),
            next_event_template: Default::default(),
            ignore_data: false,
        }
    }
}
