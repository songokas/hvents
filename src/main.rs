use anyhow::{Context, anyhow, bail};
use clap::{Parser, ValueHint};
use env_logger::Env;
use hvents::config::{
    ClientConfiguration, Config, DefinitionConfig, PoolId, QueueState, init_location, now,
};
use hvents::database::{self, KeyValueStore};
#[cfg(feature = "tiny_http")]
use hvents::events::api_listen::HttpQueue;
use hvents::events::{EventMap, EventName, EventType, Events, NextEvent, ReferencingEvent};
#[cfg(feature = "notify")]
use hvents::executors::file::file_changed_executor;
#[cfg(feature = "tiny_http")]
use hvents::executors::http::http_executor;
#[cfg(feature = "rumqttc")]
use hvents::executors::mqtt::mqtt_executor;
use hvents::executors::queue::{STATE_KEY, event_executor};
use hvents::executors::time::timed_executor;
use hvents::gherkin::definition_factory::create_definitions;
use hvents::gherkin::definition_parser::create_events_from_definitions;
#[cfg(feature = "reqwest")]
use hvents::pools::api::ClientPool;
#[cfg(feature = "tiny_http")]
use hvents::pools::http::HttpQueuePool;
#[cfg(feature = "rumqttc")]
use hvents::pools::mqtt::MqttPool;
#[cfg(feature = "handlebars")]
use hvents::renderer::load_handlebars;
use indexmap::IndexMap;
use log::{debug, info, warn};
use metrics::gauge;
#[cfg(feature = "notify")]
use notify::{RecommendedWatcher, Watcher};
use std::fs::File;
use std::path::PathBuf;
use std::{sync::mpsc, thread};

#[cfg(all(unix, feature = "evdev"))]
use hvents::executors::evdev::evdev_executor;
#[cfg(all(unix, feature = "evdev"))]
use log::error;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CliArguments {
    #[arg(required = true, help = "Path to a configuration file", value_hint = ValueHint::FilePath)]
    config: PathBuf,
    #[arg(short, long, default_value = "info")]
    verbosity: String,
}

fn main() -> Result<(), anyhow::Error> {
    let args = CliArguments::parse();
    env_logger::try_init_from_env(Env::default().default_filter_or(args.verbosity))?;
    let f = File::open(&args.config)
        .with_context(|| anyhow!("Unable to load main {} file", args.config.to_string_lossy()))?;
    let mut config: Config = serde_yaml::from_reader(f)?;
    #[cfg(all(feature = "metrics", feature = "handlebars"))]
    let recorder = if let Some(m) = config.metrics {
        otlp_metrics_exporter::install_recorder(m.service_name, m.service_version, m.instance_id)
            .into()
    } else {
        None
    };
    if let Some(l) = &config.location {
        init_location(l.latitude, l.longitude);
    }

    let definition_config: DefinitionConfig = if let Some(p) = &config.definition_path {
        let f = File::open(p).with_context(|| format!("Unable to load {}", p.to_string_lossy()))?;
        serde_yaml::from_reader(f)?
    } else {
        serde_yaml::from_str(include_str!("definitions/definitions.yaml"))?
    };
    let definitions = create_definitions(definition_config)?;

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
            if file.extension().unwrap() == "feature" {
                let (new_events, start_with) = create_events_from_definitions(file, &definitions)?;
                config.start_with.extend(start_with);
                Ok(events.merge(new_events))
            } else {
                let f = File::open(file)
                    .with_context(|| format!("Unable to load {}", file.to_string_lossy()))?;
                let e: EventMap = serde_yaml::from_reader(f)?;
                Ok(events.merge(e))
            }
        },
    )?;
    let events = events.merge(config.events);

    info!("Loaded {} events", events.len());
    gauge!("hvents.queue.loaded_events").set(events.len() as f64);

    validate_events(&events, &config.start_with, &config.http, &config.devices)?;

    #[cfg(feature = "handlebars")]
    #[allow(unused_mut)]
    let mut handlebars = load_handlebars();
    #[cfg(all(feature = "metrics", feature = "handlebars"))]
    if let Some(r) = recorder {
        hvents::renderer::add_metrics(&mut handlebars, r);
    }

    let (queue_tx, queue_rx) = mpsc::channel();
    let (timer_tx, timer_rx) = mpsc::channel();
    #[cfg(feature = "notify")]
    let (file_tx, file_rx) = mpsc::channel();
    let mut database = database::init(config.restore.as_deref());
    #[cfg(feature = "tiny_http")]
    let mut http_queue_pool = HttpQueuePool::default();
    #[cfg(feature = "rumqttc")]
    let mut mqtt_client_pool = MqttPool::default();
    #[cfg(feature = "reqwest")]
    let mut request_client_pool = ClientPool::default();

    #[cfg(feature = "notify")]
    let watcher = if events
        .iter()
        .any(|e| matches!(e.event_type, hvents::events::EventType::Watch(_)))
    {
        RecommendedWatcher::new(
            file_tx,
            notify::Config::default().with_poll_interval(core::time::Duration::from_millis(1000)),
        )?
        .into()
    } else {
        None
    };

    #[cfg(feature = "reqwest")]
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
        let mut handle_count = 0;
        #[cfg(feature = "rumqttc")]
        let mut mqtt_handles = Vec::new();
        #[cfg(feature = "rumqttc")]
        for (pool_id, mqtt_client) in config.mqtt {
            let connection = mqtt_client_pool.configure(pool_id, mqtt_client);
            let queue_tx = queue_tx.clone();
            let h = s.spawn(|| mqtt_executor(connection, &events, queue_tx));
            mqtt_handles.push(h);
            handle_count += 1;
        }

        #[cfg(all(unix, feature = "evdev"))]
        let mut device_handles = Vec::new();
        #[cfg(all(unix, feature = "evdev"))]
        for (_, device_path) in config.devices {
            let queue_tx = queue_tx.clone();
            let h = s.spawn(|| {
                let path = device_path;
                if let Err(e) = evdev_executor(&events, queue_tx, &path) {
                    error!(
                        "Reading input events from device={} failed: {e}",
                        path.to_string_lossy()
                    );
                }
            });
            device_handles.push(h);
            handle_count += 1;
        }

        #[cfg(feature = "notify")]
        let _files_changed_handle = if watcher.is_some() {
            handle_count += 1;
            s.spawn(|| file_changed_executor(&events, queue_tx.clone(), file_rx))
                .into()
        } else {
            None
        };
        #[cfg(feature = "tiny_http")]
        let mut http_handles = Vec::new();
        #[cfg(feature = "tiny_http")]
        for (pool_id, listen) in &config.http {
            let http_queue = HttpQueue::default();
            let pool_queue = http_queue.clone();
            http_queue_pool.configure(pool_id.clone(), pool_queue)?;
            let h = s.spawn(|| {
                http_executor(
                    http_queue,
                    listen,
                    &events,
                    queue_tx.clone(),
                    #[cfg(feature = "handlebars")]
                    &handlebars,
                )
            });
            http_handles.push(h);
            handle_count += 1;
        }

        handle_count += 1;

        match database.get::<QueueState>(STATE_KEY) {
            Some(saved_state) if !saved_state.is_empty() => {
                let mut state = config.initial_state;
                state.extend(saved_state);
                if let Err(e) = database.insert(STATE_KEY, &state) {
                    warn!("Unable to save state {e}");
                }
            }
            Some(_) => (),
            None => {
                if let Err(e) = database.insert(STATE_KEY, &config.initial_state) {
                    warn!("Unable to save state {e}");
                }
            }
        }
        let queue_db = database.clone();
        let _queue_handle = s.spawn(|| {
            event_executor(
                &events,
                queue_rx,
                queue_tx.clone(),
                timer_tx,
                queue_db,
                #[cfg(feature = "notify")]
                watcher,
                #[cfg(feature = "rumqttc")]
                mqtt_client_pool,
                #[cfg(feature = "reqwest")]
                request_client_pool,
                #[cfg(feature = "tiny_http")]
                http_queue_pool,
                #[cfg(feature = "handlebars")]
                &handlebars,
            )
        });

        let mut time_events = IndexMap::new();
        for ref_event in events.iter().filter(|e| e.time_event().is_some()) {
            let timer_event = database.get::<ReferencingEvent>(ref_event.event_id());
            if let Some(false) = timer_event
                .as_ref()
                .and_then(|r| r.time_event())
                .map(|t| t.expired(now()))
            {
                debug!("Restore event {}", ref_event.event_id());
                time_events.insert(
                    ref_event.event_id().to_string(),
                    timer_event.expect("time event"),
                );
            }
        }
        for name in config.start_with.iter() {
            let event_id = events
                .get_event_id(name)
                .unwrap_or_else(|| panic!("Event {name} must exit"));
            if time_events.contains_key(event_id) {
                continue;
            } else {
                let event = events
                    .get_event_by_name(name)
                    .unwrap_or_else(|| panic!("Event {name} must exit"));
                info!("Start event {}", event.name);
                queue_tx.send(event)?;
            }
        }
        let _timer_handle =
            s.spawn(|| timed_executor(&events, time_events, timer_rx, queue_tx.clone(), database));
        handle_count += 1;
        debug!("Executor count {handle_count}");
        Ok(())
    })
}

fn validate_events(
    events: &Events,
    start_events: &Vec<EventName>,
    http_listen: &IndexMap<PoolId, String>,
    devices: &IndexMap<PoolId, PathBuf>,
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
    #[cfg(feature = "tiny_http")]
    if http_listen.is_empty() {
        if let Some(e) = events
            .iter()
            .find(|e| matches!(e.event_type, EventType::ApiListen(_)))
        {
            bail!(
                "Please provide http configuration e.g. http: default: 127.0.0.1:8222 in order to use api_listen events. api_listen is provided in {}",
                e.name
            );
        }
    }

    // validate scan codes
    if devices.is_empty() {
        #[cfg(all(unix, feature = "evdev"))]
        if let Some(e) = events
            .iter()
            .find(|e| matches!(e.event_type, EventType::ScanCodeRead(_)))
        {
            bail!(
                "Please provide device configuration e.g. devices: default: /dev/input/event0 in order to use scan code read events. scan_code_read is provided in {}",
                e.name
            );
        }
    }

    // validate watch
    #[cfg(feature = "notify")]
    let watch_event = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::Watch(_)));
    #[cfg(feature = "notify")]
    let file_change_event = events
        .iter()
        .find(|e| matches!(e.event_type, EventType::FileChanged(_)));
    #[cfg(feature = "notify")]
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
