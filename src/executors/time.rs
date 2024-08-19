use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
    thread::sleep,
    time::{Duration, Instant},
};

use indexmap::IndexMap;
use log::{debug, info};

use crate::{
    config::now,
    database::KeyValueStore,
    events::{time::COOL_DOWN_DURATION, EventType, Events, ReferencingEvent},
};

pub fn timed_executor<'a>(
    events: &'a Events,
    mut events_to_execute: IndexMap<&'a str, ReferencingEvent>,
    timer_rx: Receiver<ReferencingEvent>,
    queue_tx: Sender<ReferencingEvent>,
    database: impl KeyValueStore,
) -> Result<(), anyhow::Error> {
    let mut delay_events: HashMap<&str, Instant> = HashMap::new();
    loop {
        delay_events.retain(|_, d| d.elapsed() <= COOL_DOWN_DURATION);
        for time_event in timer_rx.try_iter() {
            let event_id = events
                .get_event_id(&time_event.name)
                .unwrap_or_else(|| panic!("Event {} must exit", time_event.name));
            debug!(
                "Schedule time event with id={event_id} event={} next_event={} execute_time={}",
                time_event.name,
                time_event.next_event.as_deref().unwrap_or("unknown"),
                time_event
                    .time_event()
                    .map(|t| t.execute_time.to_string())
                    .unwrap_or_else(|| "instant".to_string())
            );
            database.insert(event_id, &time_event)?;
            events_to_execute.insert(event_id, time_event);
        }
        let now = now();
        let next_events_to_execute: Vec<(&str, ReferencingEvent)> = events_to_execute
            .iter()
            .filter_map(|(event_id, event)| {
                if !delay_events.contains_key(event.event_id()) && event.time_event()?.matches(now)
                {
                    Some((*event_id, events.get_next_event(event)?))
                } else {
                    None
                }
            })
            .collect();

        let timeout = next_events_to_execute.is_empty();
        for (event_id, mut next_event) in next_events_to_execute {
            let current_event = events_to_execute
                .shift_remove(event_id)
                .expect("event must exist");

            next_event.merge(current_event.data.clone());
            debug!("Queue next event={}", next_event.name);
            queue_tx.send(next_event)?;

            if let EventType::Repeat(_) = &current_event.event_type {
                debug!("Requeue same event={}", current_event.name);
                queue_tx.send(current_event)?;
            }

            database.remove(event_id);
            delay_events.insert(event_id, Instant::now());
        }
        if timeout {
            // cleanup old events
            for event_id in events_to_execute
                .iter()
                .filter_map(|(id, e)| e.time_event()?.expired(now).then_some(id))
            {
                info!("Removed expired event={event_id}");
                database.remove(event_id);
            }
            events_to_execute
                .retain(|_, e| !e.time_event().map(|e| e.expired(now)).unwrap_or_default());
            sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc::channel, thread::spawn};

    use chrono::{DateTime, Local};
    use serde_json::{json, Value};

    use crate::{
        config::now,
        database::Store,
        events::{
            time::{TimeEvent, TimeResult},
            EventType, NextEvent,
        },
    };

    use super::*;

    #[test]
    fn test_executor() {
        let events = [
            create_time_event(
                now(),
                "test1",
                None,
                Some("test2".to_string()),
                json!({ "test1": "text" }),
            ),
            create_time_event(
                now(),
                "test2",
                None,
                Some("test3".to_string()),
                json!({ "test2": "text" }),
            ),
            create_time_event(
                now(),
                "test3",
                None,
                Some("test4".to_string()),
                json!({ "test3": "test3_text" }),
            ),
            create_time_event(now(), "test4", None, None, json!({ "test3": "test4_text" })),
        ];
        let tevents = Events::new(events.clone().into_iter().collect());
        let (timer_tx, timer_rx) = channel();
        let (queue_tx, queue_rx) = channel();
        spawn(move || {
            timed_executor(
                &tevents,
                Default::default(),
                timer_rx,
                queue_tx,
                Store::Null,
            )
            .unwrap();
        });
        timer_tx.send(events[0].clone()).unwrap();

        sleep(Duration::from_millis(110));

        timer_tx.send(events[1].clone()).unwrap();
        sleep(Duration::from_millis(110));

        timer_tx.send(events[2].clone()).unwrap();
        timer_tx.send(events[3].clone()).unwrap();

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test2");
        assert_eq!(event.data, json!({ "test1": "text", "test2": "text" }));

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test3");
        assert_eq!(
            event.data,
            json!({ "test2": "text", "test3": "test3_text" })
        );

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test4");
        // data overwritten from test3 to test4
        assert_eq!(event.data, json!({ "test3": "test3_text" }));
        assert!(queue_rx.recv_timeout(Duration::from_millis(200)).is_err());
    }

    #[test]
    fn test_executor_overwrite_by_event_id() {
        let events = [
            create_time_event(
                now() + chrono::Duration::seconds(5),
                "test1",
                Some("abc".to_string()),
                Some("test2".to_string()),
                json!({ "test1": "text" }),
            ),
            create_time_event(
                now(),
                "test2",
                Some("abc".to_string()),
                Some("test3".to_string()),
                json!({ "test2": "text" }),
            ),
            create_time_event(now(), "test3", None, None, json!({ "test3": "text" })),
        ];
        let tevents = Events::new(events.clone().into_iter().collect());
        let (timer_tx, timer_rx) = channel();
        let (queue_tx, queue_rx) = channel();
        spawn(move || {
            timed_executor(
                &tevents,
                Default::default(),
                timer_rx,
                queue_tx,
                Store::Null,
            )
            .unwrap();
        });
        timer_tx.send(events[0].clone()).unwrap();
        timer_tx.send(events[1].clone()).unwrap();

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test3");
        assert_eq!(event.data, json!({ "test2": "text", "test3": "text" }));
        let result = queue_rx.recv_timeout(Duration::from_millis(200));
        assert!(result.is_err(), "{result:?}");
    }

    #[test]
    fn test_executor_repeat_event() {
        let events = [
            create_time_event(
                now() + chrono::Duration::seconds(5),
                "test1",
                Some("abc".to_string()),
                Some("test2".to_string()),
                json!({ "test1": "text" }),
            ),
            create_repeat_event(
                now(),
                "test2",
                Some("abc".to_string()),
                Some("test3".to_string()),
                json!({ "test2": "text" }),
            ),
            create_time_event(now(), "test3", None, None, json!({ "test3": "text" })),
        ];
        let tevents = Events::new(events.clone().into_iter().collect());
        let (timer_tx, timer_rx) = channel();
        let (queue_tx, queue_rx) = channel();
        spawn(move || {
            timed_executor(
                &tevents,
                Default::default(),
                timer_rx,
                queue_tx,
                Store::Null,
            )
            .unwrap();
        });
        timer_tx.send(events[0].clone()).unwrap();
        timer_tx.send(events[1].clone()).unwrap();

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test3");
        assert_eq!(event.data, json!({ "test2": "text", "test3": "text" }));

        // event repeated
        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test2");
        assert_eq!(event.data, json!({ "test2": "text" }));
        let result = queue_rx.recv_timeout(Duration::from_millis(200));
        assert!(result.is_err(), "{result:?}");
    }

    fn create_time_event(
        now: DateTime<Local>,
        name: &str,
        event_id: Option<String>,
        next_event: Option<String>,
        data: Value,
    ) -> ReferencingEvent {
        ReferencingEvent {
            name: name.to_string(),
            event_type: EventType::Time(TimeEvent {
                execute_time: TimeResult::Time((
                    now.naive_local().time(),
                    now.naive_local().time().to_string(),
                )),
                event_id,
            }),
            next_event: next_event.map(NextEvent::NextEvent),
            data: crate::events::data::Data::Json(data),
            ..ReferencingEvent::default()
        }
    }

    fn create_repeat_event(
        now: DateTime<Local>,
        name: &str,
        event_id: Option<String>,
        next_event: Option<String>,
        data: Value,
    ) -> ReferencingEvent {
        ReferencingEvent {
            name: name.to_string(),
            event_type: EventType::Repeat(TimeEvent {
                execute_time: TimeResult::Time((
                    now.naive_local().time(),
                    now.naive_local().time().to_string(),
                )),
                event_id,
            }),
            next_event: next_event.map(NextEvent::NextEvent),
            data: crate::events::data::Data::Json(data),
            ..ReferencingEvent::default()
        }
    }
}
