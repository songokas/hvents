use core::time::Duration;
use std::{path::Path, sync::mpsc::Sender, thread::sleep};

use evdev::{Device, InputEventKind, MiscType};
use log::{debug, info, trace, warn};
use metrics::counter;
use serde_json::json;

use crate::events::{EventType, Events, ReferencingEvent};

pub fn evdev_executor(
    events: &Events,
    queue_tx: Sender<ReferencingEvent>,
    device: &Path,
) -> anyhow::Result<()> {
    let mut max_retries = 10;
    let mut device = loop {
        match Device::open(device) {
            Ok(d) => break d,
            Err(e) if max_retries > 0 => {
                warn!("Failed to open device {e}");
                max_retries -= 1;
                sleep(Duration::from_secs(1));
                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    };

    info!("Reading events from device {device}");

    loop {
        for event in device.fetch_events()? {
            match event.kind() {
                InputEventKind::Misc(MiscType::MSC_SCAN) => {
                    counter!("hvents.evdev.incoming_events").increment(1);
                    debug!("Msc scan event {}", event.value());
                    if let Some(e) = handle_incoming_scan_code(events, event.value()) {
                        queue_tx.send(e)?;
                    }
                }
                _ => trace!("Event not handled {event:?}"),
            }
        }
    }
}

fn handle_incoming_scan_code(events: &Events, code: i32) -> Option<ReferencingEvent> {
    let event_associated = events
        .iter()
        .find_map(|ref_event| match &ref_event.event_type {
            EventType::ScanCodeRead(e) if e.matches(code) => {
                debug!(
                    "Event found event {} next event {:?}",
                    ref_event.name, ref_event.next_event
                );
                ref_event.into()
            }
            _ => None,
        })?;

    if let Some(mut event) = events.get_next_event(event_associated) {
        let mut metadata = event_associated.metadata.clone();
        metadata.merge(json!({ event_associated.name.as_str(): {"scan_code": code }}).into());
        event.metadata.merge(metadata);
        Some(event)
    } else {
        debug!(
            "Received event without further handler {}",
            event_associated.name
        );
        None
    }
}
