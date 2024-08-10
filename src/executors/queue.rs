use std::{
    sync::mpsc::{Receiver, Sender},
    thread::scope,
};

use log::{debug, error, info, warn};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rumqttc::QoS;

use crate::{
    config::now,
    events::{
        api_listen::ApiListenAction, data::Data, file_watch::WatchAction, EventType, Events,
        ReferencingEvent,
    },
    pools::{api::ClientPool, http::HttpQueuePool, mqtt::MqttPool},
    renderer::load_handlebars,
};

#[allow(clippy::too_many_arguments)]
pub fn event_executor(
    events: &Events,
    queue_rx: Receiver<ReferencingEvent>,
    queue_tx: Sender<ReferencingEvent>,
    timer_tx: Sender<ReferencingEvent>,
    mut file_watcher: Option<RecommendedWatcher>,
    mqtt_pool: MqttPool,
    client_pool: ClientPool,
    http_queue_pool: HttpQueuePool,
) -> Result<(), anyhow::Error> {
    let handlebars = load_handlebars();
    let send_next_event = |data: Data, next_event_name: Option<String>| {
        let Some(ref_event) = next_event_name else {
            return;
        };
        if let Some(mut event_to_execute) = events.get_event_by_name(&ref_event) {
            event_to_execute.merge(data);
            debug!("Queue next event={}", event_to_execute.name);
            queue_tx.send(event_to_execute).expect("event queue");
        }
    };
    scope(|s| {
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
                        if let Err(e) = c.try_subscribe(&e.topic, QoS::AtMostOnce) {
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
                            match received.data.as_bytes() {
                                Ok(b) => b,
                                Err(e) => {
                                    error!("Mqtt publish unable to obtain bytes from data {e}");
                                    continue;
                                }
                            }
                        };
                        if payload.is_empty() {
                            info!("Empty body provided for topic={}. Ignoring", e.topic);
                            continue;
                        }
                        debug!("Publish to topic={} body={payload:?}", e.topic);
                        if let Err(e) = c.try_publish(&e.topic, QoS::AtLeastOnce, e.retain, payload)
                        {
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
                EventType::ApiCall(e) => {
                    if let Some(client) = client_pool.get(&e.pool_id) {
                        s.spawn(move || match e.call_api(client, &received.data) {
                            Ok(d) => {
                                if !received.ignore_data {
                                    received.data.merge(d);
                                }
                                send_next_event(received.data, next_event_name);
                            }
                            Err(e) => {
                                error!("Failed to call api event={} {e}", received.name);
                            }
                        });
                        continue;
                    } else {
                        warn!("No client found for {}", e.pool_id);
                        continue;
                    }
                }
                EventType::ApiListen(ref e) => match e.action {
                    ApiListenAction::Start => {
                        if let Some(queue) = http_queue_pool.get(&e.pool_id) {
                            queue.lock().expect("http queue lock").replace(received);
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
                EventType::Period(e) => {
                    if !e.is_within_period(now()) {
                        debug!(
                            "Event is not scheduled for period defined in {}",
                            received.name
                        );
                        continue;
                    }
                }
                EventType::Time(e) => {
                    let Some(ref_event) = next_event_name else {
                        continue;
                    };
                    if events.has_event_by_name(&ref_event) {
                        received.event_type = EventType::Time(e.reset());
                        received.next_event = ref_event.into();
                        timer_tx.send(received).expect("timer queue");
                    }
                    continue;
                }
                EventType::Repeat(e) => {
                    let Some(ref_event) = next_event_name else {
                        continue;
                    };
                    if events.has_event_by_name(&ref_event) {
                        received.event_type = EventType::Repeat(e.reset());
                        received.next_event = ref_event.into();
                        timer_tx.send(received).expect("timer queue");
                    }
                    continue;
                }
                EventType::FileRead(ref f) => match f.read() {
                    Ok(data) => received.merge(data),
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
                EventType::Execute(c) => {
                    s.spawn(move || match c.run(&received.data) {
                        Ok(d) => {
                            if !received.ignore_data {
                                received.data.merge(d);
                            }
                            send_next_event(received.data, next_event_name);
                        }
                        Err(e) => error!("Failed to execute command {} {e}", c.command),
                    });
                    continue;
                }
                EventType::Print(e) => e.run(&received.data),
            }

            send_next_event(received.data, next_event_name);
        }
    });

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
        period::{ExecutionPeriod, PeriodEvent},
        time::TimeEvent,
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
                ignore_data: false,
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
        assert_eq!(message.event_id(), "test1");
        assert_eq!(message.name, "test1");
        assert_eq!(message.data, json!({ "test1": "text" }));
        let message = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(message.event_id(), "test1");
        assert_eq!(message.data, json!({ "test1": "text", "test3": "text" }));
        let message = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(message.event_id(), "test1");
        assert_eq!(
            message.data,
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
            event_type: match execute_period {
                Some(p) => EventType::Period(PeriodEvent::new(p)),
                None => EventType::Time(TimeEvent {
                    execute_time: "now".parse().unwrap(),
                    event_id: None,
                }),
            },
            next_event,
            data: Data::Json(data),
            next_event_template: None,
            name,
            ignore_data: false,
        }
    }
}
