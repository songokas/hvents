use std::{
    path::Path,
    sync::mpsc::{Receiver, Sender},
};

use log::{debug, error, warn};
use notify::{
    event::{AccessKind, AccessMode, CreateKind, RemoveKind},
    Event, EventKind,
};

use crate::events::{file_changed::WatchKind, EventType, Events, ReferencingEvent};

pub fn file_changed_executor(
    events: &Events,
    queue_tx: Sender<ReferencingEvent>,
    file_rx: Receiver<notify::Result<Event>>,
) -> anyhow::Result<()> {
    for event in file_rx {
        match event {
            Ok(event) => {
                // debug!("Received event {event:?}");
                let watch_kind = match event.kind {
                    EventKind::Create(CreateKind::Any | CreateKind::File) => WatchKind::Created,
                    EventKind::Access(AccessKind::Close(AccessMode::Write)) => WatchKind::Written,
                    EventKind::Remove(RemoveKind::Any | RemoveKind::File) => WatchKind::Removed,
                    _ => continue,
                };
                let Some(path) = event.paths.first() else {
                    warn!("No paths are provided for event");
                    continue;
                };
                if let Some(e) = handle_incoming(events, path, watch_kind) {
                    queue_tx.send(e)?;
                }
            }
            Err(e) => {
                error!("File changed error: {:?}", e);
            }
        }
    }
    Ok(())
}

fn handle_incoming(
    events: &Events,
    path: &Path,
    watch_kind: WatchKind,
) -> Option<ReferencingEvent> {
    debug!(
        "Received event for path {} watch kind {watch_kind}",
        path.to_string_lossy()
    );
    let change_event = events
        .iter()
        .find(|ref_event| matches!(&ref_event.event_type, EventType::FileChanged(e) if e.matches(path, watch_kind)))?;

    debug!(
        "File found event {} next event {:?}",
        change_event.name, change_event.next_event
    );

    if let Some(mut event) = change_event
        .next_event
        .as_ref()
        .and_then(|e| events.get_event_by_name(e))
    {
        event.data.merge(change_event.data.clone());
        event.into()
    } else {
        debug!(
            "Received event {} without further handler",
            change_event.name
        );
        None
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{create_dir, remove_file, File},
        io::Write,
        sync::mpsc::channel,
        thread::{sleep, spawn},
        time::Duration,
    };

    use notify::{RecommendedWatcher, RecursiveMode, Watcher};
    use serde_json::{json, Value};

    use crate::events::{data::Data, file_changed::FileChangedEvent, time::TimeEvent};

    use super::*;

    #[test]
    fn test_executor() {
        // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
        let (queue_tx, queue_rx) = channel();
        let (file_tx, file_rx) = channel();
        create_dir("/tmp/_test_change").ok();
        let event1 = FileChangedEvent {
            path: "/tmp/_test_change/1".parse().unwrap(),
            when: WatchKind::Created,
        };

        let event2 = FileChangedEvent {
            path: "/tmp/_test_change/2".parse().unwrap(),
            when: WatchKind::Written,
        };

        let event3 = FileChangedEvent {
            path: "/tmp/_test_change/1".parse().unwrap(),
            when: WatchKind::Removed,
        };
        let events = [
            create_time_event("test1", json!({ "test1": "text" })),
            create_time_event("test2", json!({ "test2": "text" })),
            create_time_event("test3", json!({ "test3": "text" })),
            ReferencingEvent {
                name: "file_create".to_string(),
                event_type: EventType::FileChanged(event1.clone()),
                next_event: "test1".to_string().into(),
                next_event_template: Default::default(),
                data: Data::Json(json!({"file_create": "data"})),
                ignore_data: false,
            },
            ReferencingEvent {
                name: "file_write".to_string(),
                event_type: EventType::FileChanged(event2.clone()),
                next_event: "test2".to_string().into(),
                next_event_template: Default::default(),
                data: Data::Json(json!({"file_write": "data"})),
                ignore_data: false,
            },
            ReferencingEvent {
                name: "file_delete".to_string(),
                event_type: EventType::FileChanged(event3.clone()),
                next_event: "test3".to_string().into(),
                next_event_template: Default::default(),
                data: Data::Json(json!({"file_delete": "data"})),
                ignore_data: false,
            },
        ];

        // make sure no file exists
        remove_file(&event1.path).ok();
        remove_file(&event2.path).ok();
        remove_file(&event3.path).ok();

        let _h = spawn(move || {
            let events = Events::new(events.into_iter().collect());
            file_changed_executor(&events, queue_tx, file_rx).unwrap();
        });
        let mut watcher = RecommendedWatcher::new(file_tx, notify::Config::default()).unwrap();

        watcher
            .watch(Path::new("/tmp/_test_change"), RecursiveMode::Recursive)
            .unwrap();

        {
            let _f = File::create(&event1.path).unwrap();
        }
        sleep(Duration::from_millis(200));
        {
            let mut f = File::create(&event2.path).unwrap();
            f.write_all(b"content").unwrap();
        }
        sleep(Duration::from_millis(200));

        remove_file(&event3.path).unwrap();

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test1");
        assert_eq!(
            event.data,
            json!({ "test1": "text", "file_create": "data" })
        );

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test2");
        assert_eq!(event.data, json!({ "test2": "text", "file_write": "data" }));

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test3");
        assert_eq!(
            event.data,
            json!({ "test3": "text", "file_delete": "data" })
        );
    }

    fn create_time_event(name: &str, data: Value) -> ReferencingEvent {
        ReferencingEvent {
            event_type: EventType::Time(TimeEvent {
                execute_time: "now".parse().unwrap(),
                event_id: None,
            }),
            next_event: None,
            data: Data::Json(data),
            next_event_template: None,
            name: name.to_string(),
            ignore_data: false,
        }
    }
}
