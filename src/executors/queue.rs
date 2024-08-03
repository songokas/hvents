use std::sync::mpsc::{Receiver, Sender};

use log::{debug, error, info, warn};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rumqttc::QoS;

use crate::{
    config::now,
    events::{
        api_listen::ApiListenAction, file_watch::WatchAction, EventType, Events, ExecutingEvent,
        ReferencingEvent, TimerMessage,
    },
    pools::{api::ClientPool, http::HttpQueuePool, mqtt::MqttPool},
    renderer::load_handlebars,
};

#[allow(clippy::too_many_arguments)]
pub fn event_executor(
    events: &Events,
    queue_rx: Receiver<ReferencingEvent>,
    queue_tx: Sender<ReferencingEvent>,
    timer_tx: Sender<TimerMessage>,
    mut file_watcher: Option<RecommendedWatcher>,
    mqtt_pool: MqttPool,
    client_pool: ClientPool,
    http_queue_pool: HttpQueuePool,
) -> Result<(), anyhow::Error> {
    let handlebars = load_handlebars();

    for mut received in queue_rx {
        let next_event_name = match (&received.next_event, &received.next_event_template) {
            (Some(s), _) => Some(s.clone()),
            (None, Some(s)) => match handlebars.render_template(s, &received.data) {
                Ok(s) => Some(s),
                Err(e) => {
                    error!("Failed to render event template {e}");
                    None
                }
            },
            (None, None) => None,
        };
        match received.event_type {
            EventType::MqttSubscribe(e) => {
                if let Some(c) = mqtt_pool.get(&e.pool_id) {
                    if let Err(e) = c.subscribe(&e.topic, QoS::AtMostOnce) {
                        error!("Failed to subscribe {e}")
                    } else {
                        info!("Subscribed to {}", e.topic);
                    }
                } else {
                    warn!(
                        "Mqtt subscribed for {}, but no client is defined. Ignoring",
                        e.topic
                    );
                }
                // subscription events begin in mqtt_executor
                continue;
            }
            EventType::MqttPublish(ref e) => {
                if let Some(c) = mqtt_pool.get(&e.pool_id) {
                    let payload = if let Some(template) = &e.template {
                        let mut payload = Vec::default();
                        if let Err(e) = handlebars.render_template_to_write(
                            template,
                            &received.data,
                            &mut payload,
                        ) {
                            error!("Failed to render template event={} {e}", received.name);
                            continue;
                        }
                        payload.into()
                    } else {
                        received.data.as_bytes()
                    };
                    if payload.is_empty() {
                        info!("Empty body provided for topic={}. Ignoring", e.topic);
                        continue;
                    }
                    debug!("Publish to topic={} body={payload:?}", e.topic);
                    if let Err(e) = c.try_publish(&e.topic, QoS::AtLeastOnce, e.retain, payload) {
                        error!("Failed to publish {e}");
                        continue;
                    }
                } else {
                    warn!(
                        "Mqtt publish for {} received, but not client is defined. Ignoring",
                        e.topic
                    );
                }
            }
            EventType::ApiCall(ref e) => {
                if let Some(client) = client_pool.get(&e.pool_id) {
                    match e.call_api(client, &received.data) {
                        Ok(d) => received.data.merge(d),
                        Err(e) => {
                            error!("Failed to call api event={} {e}", received.name);
                            continue;
                        }
                    }
                } else {
                    warn!("No client found for {}", e.pool_id);
                    continue;
                }
            }
            EventType::ApiListen(ref e) => match e.action {
                ApiListenAction::Start => {
                    if let Some(queue) = http_queue_pool.get(&e.pool_id) {
                        queue.lock().expect("http queue lock").insert(received);
                    } else {
                        warn!("No http queue found for {}", e.pool_id);
                    }
                    // listen events begin in http executor
                    continue;
                }
                ApiListenAction::Stop => {
                    if let Some(queue) = http_queue_pool.get(&e.pool_id) {
                        queue
                            .lock()
                            .expect("http queue lock")
                            .shift_remove(received.name.as_str());
                    } else {
                        warn!("No http queue found for {}", e.pool_id);
                    }
                }
            },
            EventType::Time(e) => {
                let Some(ref_event) = next_event_name else {
                    continue;
                };

                if !e.can_execute(now()) {
                    debug!(
                        "Event {ref_event} not scheduled for period defined in {}",
                        received.name
                    );
                    continue;
                }

                if let Some(mut event_to_execute) = events.get_event_by_name(&ref_event) {
                    debug!(
                        "Schedule event with id={} from event={} execute_time={}",
                        ref_event,
                        received.name,
                        e.execute_time
                            .as_ref()
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| "instant".to_string())
                    );
                    event_to_execute.data.merge(received.data.clone());
                    timer_tx.send(TimerMessage {
                        executing_event: ExecutingEvent {
                            name: received.name,
                            event_id: ref_event,
                            time_event: e.reset(),
                            next_event: received.next_event,
                            next_event_template: received.next_event_template,
                            data: received.data,
                        },
                        event_to_execute,
                    })?;
                }
                continue;
            }
            EventType::FileRead(ref f) => match f.read() {
                Ok(data) => received.data.merge(data),
                Err(e) => {
                    error!("Error while reading file {e}");
                    continue;
                }
            },
            EventType::FileWrite(ref f) => {
                if let Err(e) = f.write(&received.data) {
                    error!("Error while writing file {e}");
                    continue;
                }
            }
            // these events are handled in file change executor
            EventType::FileChanged(_) => (),
            EventType::Watch(f) => match f.action {
                WatchAction::Start => {
                    let mode = if f.recursive {
                        RecursiveMode::Recursive
                    } else {
                        RecursiveMode::NonRecursive
                    };
                    if let Err(e) = file_watcher
                        .as_mut()
                        .map(|w| w.watch(&f.path, mode))
                        .transpose()
                    {
                        error!("Unable to watch {} {e}", f.path.to_string_lossy());
                    }
                }
                WatchAction::Stop => {
                    if let Err(e) = file_watcher
                        .as_mut()
                        .map(|w| w.unwatch(&f.path))
                        .transpose()
                    {
                        error!("Unable to unwatch {} {e}", f.path.to_string_lossy());
                    }
                }
            },
            EventType::Execute(c) => match c.run(&received.data) {
                Ok(d) => received.data.merge(d),
                Err(e) => error!("Failed to execute command {} {e}", c.command),
            },
            EventType::Print(e) => e.run(&received.data),
        }

        let Some(ref_event) = next_event_name else {
            continue;
        };
        if let Some(mut event_to_execute) = events.get_event_by_name(&ref_event) {
            event_to_execute.data.merge(received.data);
            debug!("Queue next event {}", event_to_execute.name);
            queue_tx.send(event_to_execute)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::{sync::mpsc::channel, thread::spawn};

    use serde_json::{json, Value};

    use crate::events::{
        data::Data,
        mqtt_publish::MqttPublishEvent,
        time::{ExecutionPeriod, TimeEvent},
    };

    use super::*;

    #[test]
    fn test_executor() {
        let (timer_tx, timer_rx) = channel();
        let (queue_tx, queue_rx) = channel();

        let events = [
            create_event(
                "test1".to_string(),
                Some("test2".to_string()),
                None,
                json!({ "test1": "text" }),
            ),
            create_event("test2".to_string(), None, None, Default::default()),
            create_event(
                "test3".to_string(),
                Some("test1".to_string()),
                ExecutionPeriod {
                    from: "1 second ago".parse().unwrap(),
                    to: "in 1 second".parse().unwrap(),
                }
                .into(),
                json!({ "test3": "text" }),
            ),
            create_event(
                "test4".to_string(),
                Some("test3".to_string()),
                ExecutionPeriod {
                    from: "tomorrow".parse().unwrap(),
                    to: "tomorrow".parse().unwrap(),
                }
                .into(),
                json!({ "test4": "text" }),
            ),
            ReferencingEvent {
                event_type: EventType::MqttPublish(MqttPublishEvent {
                    topic: "1".to_string(),
                    pool_id: Default::default(),
                    template: Default::default(),
                    retain: false,
                }),
                next_event: Some("test1".to_string()),
                data: Data::Json(json!({ "test1": "new_text", "test5": "text" })),
                next_event_template: None,
                name: "test5".to_string(),
            },
        ];

        spawn(move || {
            for event in events.iter() {
                queue_tx.send(event.clone()).unwrap();
            }
            let events = Events::new(events.into_iter().collect());
            event_executor(
                &events,
                queue_rx,
                queue_tx.clone(),
                timer_tx,
                None,
                MqttPool::default(),
                ClientPool::default(),
                HttpQueuePool::default(),
            )
            .unwrap();
        });

        let message = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(message.executing_event.event_id, "test2");
        assert_eq!(message.event_to_execute.data, json!({ "test1": "text" }));
        let message = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(message.executing_event.event_id, "test1");
        assert_eq!(
            message.event_to_execute.data,
            json!({ "test1": "text", "test3": "text" })
        );
        let message = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(message.executing_event.event_id, "test2");
        assert_eq!(
            message.event_to_execute.data,
            json!({ "test1": "new_text", "test5": "text" })
        );
        let result = timer_rx.recv_timeout(Duration::from_millis(200));
        assert!(result.is_err());
    }

    fn create_event(
        name: String,
        next_event: Option<String>,
        execute_period: Option<ExecutionPeriod>,
        data: Value,
    ) -> ReferencingEvent {
        ReferencingEvent {
            event_type: EventType::Time(TimeEvent {
                execute_time: None,
                execute_period,
            }),
            next_event,
            data: Data::Json(data),
            next_event_template: None,
            name,
        }
    }
}
