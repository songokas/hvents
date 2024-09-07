use std::{
    sync::mpsc::{Receiver, Sender},
    thread::{scope, Builder},
};

use indexmap::IndexMap;
use log::{debug, error, info, warn};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rumqttc::QoS;

use crate::{
    config::now,
    events::{
        api_listen::ApiListenAction,
        data::{Data, Metadata},
        file_watch::WatchAction,
        EventType, Events, NextEvent, ReferencingEvent,
    },
    pools::{api::ClientPool, http::HttpQueuePool, mqtt::MqttPool},
    renderer::{load_handlebars, TemplateData},
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
    let mut state: IndexMap<String, String> = IndexMap::new();
    let send_next_event = |data: Data, metadata: Metadata, next_event_name: Option<String>| {
        let Some(ref_event) = next_event_name else {
            return;
        };
        if let Some(mut event_to_execute) = events.get_event_by_name(&ref_event) {
            event_to_execute.merge(data);
            event_to_execute.metadata.merge(metadata);
            debug!("Queue next event={}", event_to_execute.name);
            queue_tx.send(event_to_execute).expect("event queue");
        }
    };
    scope(|thread_scope| {
        'main: for mut received in queue_rx {
            if let Some(key) = received.state.as_ref().and_then(|s| s.count.as_deref()) {
                state
                    .entry(key.to_string())
                    .and_modify(|e| *e = (e.parse::<u64>().unwrap_or(0) + 1).to_string())
                    .or_insert_with(|| 0.to_string());
            }
            if let Some(map) = received.state.as_ref().map(|s| &s.replace) {
                state.extend(map.clone());
            }

            let template_data = TemplateData {
                data: &received.data,
                metadata: &received.metadata,
                state: &state,
            };

            let next_event_name = match &received.next_event {
                Some(NextEvent::Template(s)) => {
                    match handlebars.render_template(s, &template_data) {
                        Ok(s) => Some(s),
                        Err(e) => {
                            error!("Failed to render event template {e}");
                            None
                        }
                    }
                }
                Some(NextEvent::Name(s)) => Some(s.clone()),
                None => None,
            };

            if next_event_name.as_ref() == Some(&received.name) {
                warn!(
                    "Current event={} and next event must not be the same event. Ignoring",
                    received.name
                );
                continue;
            }

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
                            "Mqtt subscribed for {} expected, but no client is defined. Ignoring",
                            e.topic
                        );
                    }
                    // subscription events begin in mqtt_executor
                    continue;
                }
                EventType::MqttUnsubscribe(e) => {
                    if let Some(c) = mqtt_pool.get(&e.pool_id) {
                        if let Err(e) = c.try_unsubscribe(&e.topic) {
                            error!("Failed to subscribe {e}")
                        }
                    } else {
                        warn!(
                            "Mqtt unsubscribe for {} expected, but no client is defined. Ignoring",
                            e.topic
                        );
                    }
                }
                EventType::MqttPublish(ref e) => {
                    if let Some(c) = mqtt_pool.get(&e.pool_id) {
                        let topic = match handlebars.render_template(&e.topic, &template_data) {
                            Ok(t) if !t.trim().is_empty() => t,
                            Ok(_) => {
                                info!("Empty topic provided for event={}. Ignoring", received.name);
                                continue;
                            }
                            Err(e) => {
                                error!("Failed to render template event={} {e}", received.name);
                                continue;
                            }
                        };
                        let payload = if let Some(template) = &e.body {
                            let mut payload = Vec::default();
                            if let Err(e) = handlebars.render_template_to_write(
                                template,
                                &template_data,
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
                            info!("Empty body provided for topic={}. Ignoring", topic);
                            continue;
                        }
                        debug!("Publish to topic={} body={payload:?}", topic);
                        if let Err(e) = c.try_publish(&topic, QoS::AtLeastOnce, e.retain, payload) {
                            error!("Failed to publish topic={topic} {e}");
                            continue;
                        }
                    } else {
                        warn!(
                            "Mqtt publish for {} received, but not client is defined. Ignoring",
                            e.topic
                        );
                    }
                }
                EventType::ApiCall(mut e) => {
                    if let Some(client) = client_pool.get(&e.pool_id) {
                        match handlebars.render_template(&e.url, &template_data) {
                            Ok(url) => e.url = url,
                            Err(e) => {
                                error!("Failed to render url template {e}");
                                continue 'main;
                            }
                        };
                        let result = Builder::new()
                            .name(format!("api_call {}", e.url))
                            .spawn_scoped(thread_scope, move || {
                                match e.call_api(client, &received.data, &received.name) {
                                    Ok((d, m)) => {
                                        received.data.merge_with_policy(d, received.merge_data);
                                        received.metadata.merge(m);
                                        send_next_event(
                                            received.data,
                                            received.metadata,
                                            next_event_name,
                                        );
                                    }
                                    Err(e) => {
                                        error!("Failed to call api event={} {e}", received.name);
                                    }
                                }
                            });
                        if let Err(e) = result {
                            error!("Unable to call api {e}");
                        }
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
                    received.event_type = EventType::Time(e.reset());
                    timer_tx.send(received).expect("timer queue");
                    continue;
                }
                EventType::Repeat(e) => {
                    received.event_type = EventType::Repeat(e.reset());
                    timer_tx.send(received).expect("timer queue");
                    continue;
                }
                EventType::FileRead(ref f) => match f.read() {
                    Ok((d, m)) => {
                        received.merge(d);
                        received.metadata.merge(m);
                    }
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
                EventType::FileChanged(_) => continue,
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
                EventType::Execute(mut c) => {
                    let args = &mut c.args;
                    for (index, template) in &c.replace_args {
                        match handlebars.render_template(template, &template_data) {
                            Ok(a) if args.get(*index).is_some() => args[*index] = a,
                            Ok(_) => {
                                warn!("Failed to replace argument at index {index} {template}");
                                continue 'main;
                            }
                            Err(e) => {
                                warn!("Failed to render command argument {template} {e}");
                                continue 'main;
                            }
                        };
                    }
                    let result = Builder::new()
                        .name(format!("command {}", c.command))
                        .spawn_scoped(thread_scope, move || match c.run(&received.data) {
                            Ok((d, m)) => {
                                received.data.merge_with_policy(d, received.merge_data);
                                received.metadata.merge(m);
                                send_next_event(received.data, received.metadata, next_event_name);
                            }
                            Err(e) => error!("Failed to execute command {} {e}", c.command),
                        });
                    if let Err(e) = result {
                        error!("Unable to run command {e}");
                    }
                    continue;
                }
                EventType::Print(e) => e.run(&received.data),
                EventType::Pass => (),
                // events begin in evdev executor
                #[cfg(target_os = "linux")]
                EventType::ScanCodeRead(_) => continue,
            }

            send_next_event(received.data, received.metadata, next_event_name);
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
        StateData,
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
                    body: Default::default(),
                    retain: false,
                }),
                next_event: Some("test1".into()),
                data: Data::Json(json!({ "test1": "new_text", "test5": "text" })),
                name: "test5".to_string(),
                ..ReferencingEvent::default()
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

        let event = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.event_id(), "test1");
        assert_eq!(event.name, "test1");
        assert_eq!(event.data, json!({ "test1": "text" }));
        let event = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test2");
        let event = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test1");
        assert_eq!(event.data, json!({ "test1": "text", "test3": "text" }));
        let event = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.event_id(), "test1");
        assert_eq!(event.data, json!({ "test1": "new_text", "test5": "text" }));
        let result = timer_rx.recv_timeout(Duration::from_millis(200));
        assert!(result.is_err());
    }

    #[test]
    fn test_next_event() {
        let (timer_tx, timer_rx) = channel();
        let (queue_tx, queue_rx) = channel();

        let events = [
            ReferencingEvent {
                event_type: EventType::Time(TimeEvent {
                    execute_time: "now".parse().unwrap(),
                    event_id: None,
                }),
                name: "test1".to_string(),
                state: StateData {
                    replace: indexmap::indexmap! {
                    "next_event".to_string() => "test3".to_string(),
                    },
                    count: None,
                }
                .into(),
                next_event: NextEvent::from("test2").into(),
                ..ReferencingEvent::default()
            },
            ReferencingEvent {
                event_type: EventType::Time(TimeEvent {
                    execute_time: "now".parse().unwrap(),
                    event_id: None,
                }),
                name: "test2".to_string(),
                next_event: NextEvent::Template("{{state.next_event}}".to_string()).into(),
                ..ReferencingEvent::default()
            },
            ReferencingEvent {
                event_type: EventType::Time(TimeEvent {
                    execute_time: "now".parse().unwrap(),
                    event_id: None,
                }),
                name: "test3".to_string(),
                ..ReferencingEvent::default()
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

        let event = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test1");
        let event = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test2");
        let event = timer_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test3");
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
            next_event: next_event.map(NextEvent::Name),
            data: Data::Json(data),
            name,
            ..ReferencingEvent::default()
        }
    }
}
