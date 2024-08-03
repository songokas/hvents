use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
    thread::sleep,
    time::{Duration, Instant},
};

use indexmap::IndexSet;
use log::{debug, info};

use crate::{
    config::now,
    database::KeyValueStore,
    events::{time::COOL_DOWN_DURATION, EventName, EventType, ReferencingEvent, TimerMessage},
};

pub fn timed_executor(
    mut events_to_execute: IndexSet<TimerMessage>,
    timer_rx: Receiver<TimerMessage>,
    queue_tx: Sender<ReferencingEvent>,
    database: impl KeyValueStore,
) -> Result<(), anyhow::Error> {
    let mut delay_events: HashMap<EventName, Instant> = HashMap::new();
    loop {
        delay_events.retain(|_, d| d.elapsed() <= COOL_DOWN_DURATION);
        for timer_message in timer_rx.try_iter() {
            database.insert(&timer_message.executing_event.event_id, &timer_message)?;
            events_to_execute.insert(timer_message);
        }
        let now = now();
        let event_names_to_execute: Vec<String> = events_to_execute
            .iter()
            .filter_map(|event| {
                (!delay_events.contains_key(&event.executing_event.event_id)
                    && event.executing_event.time_event.matches(now))
                .then_some(event.executing_event.event_id.clone())
            })
            .collect();

        let timeout = event_names_to_execute.is_empty();
        for event_id in event_names_to_execute {
            let message = events_to_execute
                .shift_take(event_id.as_str())
                .expect("event must exist");

            debug!("Queue next event {}", message.event_to_execute.name);
            queue_tx.send(message.event_to_execute)?;

            let reschedule_event = message.executing_event.time_event.reset();
            if !reschedule_event.expired(now) {
                debug!("Requeue same event {}", message.executing_event.name);
                queue_tx.send(ReferencingEvent {
                    name: message.executing_event.name,
                    event_type: EventType::Time(reschedule_event),
                    next_event: message.executing_event.next_event,
                    next_event_template: message.executing_event.next_event_template,
                    data: message.executing_event.data,
                })?;
            } else {
                debug!(
                    "Ignoring event {} since its expired {:?}",
                    message.executing_event.name, reschedule_event.execute_time
                );
            }
            database.remove(&event_id);
            delay_events.insert(event_id, Instant::now());
        }
        if timeout {
            // cleanup old events
            let current_size = events_to_execute.len();
            for e in events_to_execute
                .iter()
                .filter(|m| m.executing_event.time_event.expired(now))
            {
                database.remove(&e.executing_event.event_id);
            }
            events_to_execute.retain(|m| !m.executing_event.time_event.expired(now));
            if current_size > events_to_execute.len() {
                info!(
                    "Removed {} expired events",
                    current_size - events_to_execute.len()
                )
            }
            sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc::channel, thread::spawn};

    use chrono::{DateTime, Local};

    use crate::{
        config::now,
        database::Store,
        events::{
            time::{TimeEvent, TimeResult},
            EventType, ExecutingEvent,
        },
    };

    use super::*;

    #[test]
    fn test_executor() {
        let (timer_tx, timer_rx) = channel();
        let (queue_tx, queue_rx) = channel();
        spawn(|| {
            timed_executor(Default::default(), timer_rx, queue_tx, Store::Null).unwrap();
        });
        timer_tx
            .send(create_time_message(
                now(),
                "test1",
                "test2",
                "test2",
                "test3",
            ))
            .unwrap();

        sleep(Duration::from_millis(110));

        timer_tx
            .send(create_time_message(
                now(),
                "test4",
                "test5",
                "test5",
                "test6",
            ))
            .unwrap();
        sleep(Duration::from_millis(110));

        timer_tx
            .send(create_time_message(
                now(),
                "test7",
                "test1",
                "test1",
                "test8",
            ))
            .unwrap();
        timer_tx
            .send(create_time_message(
                now() - Duration::from_secs(10),
                "test9",
                "test10",
                "test10",
                "test11",
            ))
            .unwrap();
        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test2");
        // automatic reschedule
        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test1");
        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test5");
        // automatic reschedule
        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test4");

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test1");
        // automatic reschedule
        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test7");
        assert!(queue_rx.recv_timeout(Duration::from_millis(200)).is_err());
    }

    fn create_time_message(
        now: DateTime<Local>,
        name: &str,
        executing_event_id: &str,
        next_event_name: &str,
        next_next_event_name: &str,
    ) -> TimerMessage {
        TimerMessage {
            executing_event: ExecutingEvent {
                name: name.to_string(),
                event_id: executing_event_id.to_string(),
                time_event: TimeEvent {
                    execute_time: TimeResult::Time((
                        now.naive_local().time(),
                        now.naive_local().time().to_string(),
                    ))
                    .into(),
                    execute_period: None,
                },
                next_event: next_event_name.to_string().into(),
                next_event_template: None,
                data: Default::default(),
            },
            event_to_execute: ReferencingEvent {
                name: next_event_name.to_string(),
                event_type: EventType::Time(TimeEvent {
                    execute_time: None,
                    execute_period: None,
                }),
                next_event: format!("ref_{next_next_event_name}").into(),
                data: Default::default(),
                next_event_template: None,
            },
        }
    }
}
