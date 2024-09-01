use anyhow::{anyhow, bail, Context};
use core::time::Duration;
use env_logger::Env;
use hvents::config::{init_location, ClientConfiguration, Config, PoolId};
use hvents::database::{self, KeyValueStore};
use hvents::events::api_listen::HttpQueue;
use hvents::events::{EventMap, EventName, EventType, Events, NextEvent, ReferencingEvent};
use hvents::executors::file::file_changed_executor;
use hvents::executors::http::http_executor;
use hvents::executors::mqtt::mqtt_executor;
use hvents::executors::queue::event_executor;
use hvents::executors::time::timed_executor;
use hvents::pools::api::ClientPool;
use hvents::pools::http::HttpQueuePool;
use hvents::pools::mqtt::MqttPool;
use indexmap::IndexMap;
use log::{debug, info};
use notify::{RecommendedWatcher, Watcher};
use std::env::args;
use std::fs::File;
use std::{sync::mpsc, thread};

fn main() -> Result<(), anyhow::Error> {
    env_logger::try_init_from_env(Env::default().default_filter_or("info"))?;
    let config_file = args()
        .nth(1)
        .ok_or_else(|| anyhow!("Provide configuration file as argument"))?;
    let f = File::open(&config_file)
        .with_context(|| anyhow!("Unable to load main {config_file} file"))?;
    let config: Config = serde_yaml::from_reader(f)?;

    if let Some(l) = &config.location {
        init_location(l.latitude, l.longitude);
    }

    let events = config.groups.iter().try_fold(
        Events::default(),
        |events, (prefix, file)| -> Result<Events, anyhow::Error> {
            info!(
                "Loading file {} with prefix {prefix}",
                file.to_string_lossy()
            );
            let f = File::open(file)
                .with_context(|| format!("Unable to load {}", file.to_string_lossy()))?;
            let e: EventMap = serde_yaml::from_reader(f)?;
            Ok(events.merge_with_prefix(e, prefix))
        },
    )?;
    let events = config.event_files.iter().try_fold(
        events,
        |events, file| -> Result<Events, anyhow::Error> {
            info!("Loading file {}", file.to_string_lossy());
            let f = File::open(file)
                .with_context(|| format!("Unable to load {}", file.to_string_lossy()))?;
            let e: EventMap = serde_yaml::from_reader(f)?;
            Ok(events.merge(e))
        },
    )?;
    let events = events.merge(config.events);

    info!("Loaded {} events", events.len());

    validate_events(&events, &config.start_with, &config.http)?;

    let (queue_tx, queue_rx) = mpsc::channel();
    let (timer_tx, timer_rx) = mpsc::channel();
    let (file_tx, file_rx) = mpsc::channel();
    let database = database::init(config.restore.as_deref());
    let mut http_queue_pool = HttpQueuePool::default();
    let mut mqtt_client_pool = MqttPool::default();
    let mut request_client_pool = ClientPool::default();

    let watcher = if events
        .iter()
        .any(|e| matches!(e.event_type, hvents::events::EventType::Watch(_)))
    {
        RecommendedWatcher::new(
            file_tx,
            notify::Config::default().with_poll_interval(Duration::from_millis(1000)),
        )?
        .into()
    } else {
        None
    };

    if config.api.is_empty() {
        request_client_pool.configure(
            "default".to_string(),
            &ClientConfiguration {
                default_headers: Default::default(),
            },
        )?;
    } else {
        for (pool_id, config) in &config.api {
            request_client_pool.configure(pool_id.clone(), config)?;
        }
    }

    thread::scope(|s| -> Result<(), anyhow::Error> {
        let mut mqtt_handles = Vec::new();
        for (pool_id, mqtt_client) in config.mqtt {
            let connection = mqtt_client_pool.configure(pool_id, mqtt_client);
            let queue_tx = queue_tx.clone();
            let h = s.spawn(|| mqtt_executor(connection, &events, queue_tx));
            mqtt_handles.push(h);
        }

        let _files_changed_handle = if watcher.is_some() {
            s.spawn(|| file_changed_executor(&events, queue_tx.clone(), file_rx))
                .into()
        } else {
            None
        };
        let mut http_handles = Vec::new();
        for (pool_id, listen) in &config.http {
            let http_queue = HttpQueue::default();
            let pool_queue = http_queue.clone();
            http_queue_pool.configure(pool_id.clone(), pool_queue)?;
            let h = s.spawn(|| http_executor(http_queue, listen, &events, queue_tx.clone()));
            http_handles.push(h);
        }

        let _queue_handle = s.spawn(|| {
            event_executor(
                &events,
                queue_rx,
                queue_tx.clone(),
                timer_tx,
                watcher,
                mqtt_client_pool,
                request_client_pool,
                http_queue_pool,
            )
        });

        let mut time_events = IndexMap::new();
        for name in config.start_with.iter() {
            let event = events
                .get_event_by_name(name)
                .unwrap_or_else(|| panic!("Event {name} must exit"));
            let event_id = events
                .get_event_id(name)
                .unwrap_or_else(|| panic!("Event {name} must exit"));
            if let Some(timer_event) = database.get::<ReferencingEvent>(event_id) {
                debug!("Restore event {}", event_id);
                time_events.insert(event_id, timer_event);
            } else {
                info!("Start event {}", event.name);
                queue_tx.send(event)?;
            }
        }
        let _timer_handle =
            s.spawn(|| timed_executor(&events, time_events, timer_rx, queue_tx.clone(), database));

        Ok(())
    })
}

fn validate_events(
    events: &Events,
    start_events: &Vec<EventName>,
    http_listen: &IndexMap<PoolId, String>,
) -> anyhow::Result<()> {
    if events.is_empty() {
        bail!("No events specified, please define at least one event");
    }
    // validate references
    for event in events.iter() {
        let Some(NextEvent::Name(name)) = &event.next_event else {
            continue;
        };
        if !events.has_event_by_name(name) {
            bail!(
                "Event with name {name} not found, referenced in {}.event",
                event.name
            );
        }
    }

    // validate startup
    for name in start_events {
        if !events.has_event_by_name(name) {
            bail!("Event with name {name} not found, referenced in start_with");
        }
    }

    // validate http
    if http_listen.is_empty() {
        if let Some(e) = events
            .iter()
            .find(|e| matches!(e.event_type, EventType::ApiListen(_)))
        {
            bail!("Please provide http configuration e.g. http: default: 127.0.0.1:8222 in order to use api_listen events. api_listen is provided in {}", e.name);
        }
    }

    // validate watch
    let watch_event = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Watch(_)));
    let file_change_event = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::FileChanged(_)));
    if watch_event.is_some() != file_change_event.is_some() {
        if let Some(w) = watch_event {
            bail!(
                "Watch event {} is defined, but no file_changed events found. Please define at least one file_change event",
                w.name
            );
        }
        if let Some(w) = file_change_event {
            bail!(
                "File change event {} is defined, but no watch events found. Please define at least one watch event",
                w.name
            );
        }
    }
    Ok(())
}
