use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use gherkin::Scenario;
use gherkin::{Feature, GherkinEnv};
use indexmap::IndexMap;
use log::debug;

use crate::{
    events::{EventMap, EventName, NextEvent},
    gherkin::event_factory::EventFactory,
};

pub fn create_events_from_definitions(
    definition_file_path: &Path,
    definitions: &[Box<dyn EventFactory>],
) -> Result<(EventMap, Vec<EventName>)> {
    let env = GherkinEnv::default();
    let feature = Feature::parse_path(definition_file_path, env)?;

    let mut event_map = EventMap::new();
    let mut start_with = Vec::new();

    for scenario in feature.scenarios {
        if !scenario.examples.is_empty() {
            for example in &scenario.examples {
                let mut rows: std::vec::IntoIter<Vec<String>> = example
                    .table
                    .clone()
                    .ok_or_else(|| anyhow!("Expected a table in a definition"))?
                    .rows
                    .into_iter();
                let value_names = rows
                    .next()
                    .ok_or_else(|| anyhow!("Table names are required"))?;
                let value_fields = rows.collect::<Vec<_>>();
                for values in value_fields {
                    let example_fields = value_names.iter().zip(values.iter()).collect::<IndexMap<
                        &String,
                        &String,
                    >>(
                    );
                    let (events, start_with_event) =
                        parse_steps(&scenario, definitions, example_fields)?;
                    if let Some(event_name) = start_with_event {
                        start_with.push(event_name.clone());
                    }
                    event_map.extend(events);
                }
            }
        } else {
            let (events, start_with_event) =
                parse_steps(&scenario, definitions, Default::default())?;
            if let Some(event_name) = start_with_event {
                start_with.push(event_name.clone());
            }
            event_map.extend(events);
        }
    }
    Ok((event_map, start_with))
}

fn parse_steps(
    scenario: &Scenario,
    definitions: &[Box<dyn EventFactory>],
    example_fields: IndexMap<&String, &String>,
) -> Result<(EventMap, Option<EventName>)> {
    let mut event_map = EventMap::new();
    let mut start_with = None;
    let mut iter = scenario.steps.iter().peekable();
    let no_start_tag = scenario.tags.contains(&"no_start".to_string());
    let index = example_fields
        .iter()
        .map(|(_, v)| v.to_string())
        .collect::<Vec<_>>()
        .join(",");

    while let Some(step) = iter.next() {
        let next_step = iter.peek();
        let mut value = step.value.clone();
        for (k, v) in &example_fields {
            value = value.replace(format!("<{k}>").as_str(), v);
        }
        let event_name = format!("{} {}{index}", scenario.name, value);
        let mut event = None;
        for d in definitions {
            if let Some(e) = d.create_event(value.clone()).context(step.value.clone())? {
                event = Some(e);
                break;
            }
        }

        let Some(mut event) = event else {
            bail!(
                "Scenario `{}` Step `{}` not handled",
                scenario.name,
                step.value
            )
        };
        if let Some(next_event_value) = next_step.map(|s| {
            let mut value = s.value.clone();
            for (k, v) in &example_fields {
                value = value.replace(format!("<{k}>").as_str(), v);
            }
            value
        }) {
            event.next_event =
                NextEvent::Name(format!("{} {}{index}", scenario.name, next_event_value)).into();
        }
        event.name = event_name.clone();
        if step.keyword.trim() == "Given" {
            if no_start_tag {
                debug!("Not starting event {event_name} because of no_start tag");
            } else {
                start_with = event_name.clone().into();
            }
        }
        event_map.insert(event_name, event);
    }
    Ok((event_map, start_with))
}

#[cfg(test)]
mod tests {
    use crate::config::INSTA_OUTPUT;
    use crate::gherkin::definition_factory::create_definitions;
    use insta::assert_yaml_snapshot;

    use super::*;

    #[test]
    fn test_definition_parser() {
        let config = serde_yaml::from_str(include_str!("../definitions/definitions.yaml")).unwrap();
        let definitions = create_definitions(config).unwrap();
        let events =
            create_events_from_definitions(Path::new("examples/state.feature"), &definitions)
                .unwrap();
        INSTA_OUTPUT.set(true);
        assert_yaml_snapshot!(events);
        INSTA_OUTPUT.set(false);
    }

    #[test]
    fn test_api_definition_parser() {
        let config = serde_yaml::from_str(include_str!("../definitions/definitions.yaml")).unwrap();
        let definitions = create_definitions(config).unwrap();
        let events =
            create_events_from_definitions(Path::new("examples/api.feature"), &definitions)
                .unwrap();
        INSTA_OUTPUT.set(true);
        assert_yaml_snapshot!(events);
        INSTA_OUTPUT.set(false);
    }

    #[test]
    fn test_file_definition_parser() {
        let config = serde_yaml::from_str(include_str!("../definitions/definitions.yaml")).unwrap();
        let definitions = create_definitions(config).unwrap();
        let events =
            create_events_from_definitions(Path::new("examples/file.feature"), &definitions)
                .unwrap();
        INSTA_OUTPUT.set(true);
        assert_yaml_snapshot!(events);
        INSTA_OUTPUT.set(false);
    }
}
