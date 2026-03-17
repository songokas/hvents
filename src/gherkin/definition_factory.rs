use anyhow::Result;

use crate::{config::DefinitionConfig, gherkin::event_factory::EventFactory};

pub fn create_definitions(config: DefinitionConfig) -> Result<Vec<Box<dyn EventFactory>>> {
    let mut definitions: Vec<Box<dyn EventFactory>> = vec![];
    if let Some(config) = config.get("repeat") {
        definitions.push(Box::new(
            crate::definitions::repeat::RepeatDefinition::from_config(config.clone())?,
        ));
    };
    if let Some(config) = config.get("once") {
        definitions.push(Box::new(
            crate::definitions::once::OnceDefinition::from_config(config.clone())?,
        ));
    };

    if let Some(config) = config.get("comparison") {
        definitions.push(Box::new(
            crate::definitions::data_comparison::DataComparisonDefinition::from_config(
                config.clone(),
            )?,
        ));
    };

    if let Some(config) = config.get("state_modifications") {
        definitions.push(Box::new(
            crate::definitions::state_modification::StateModificationDefinition::from_config(
                config.clone(),
            )?,
        ));
    };

    #[cfg(feature = "rumqttc")]
    if let Some(config) = config.get("mqtt_publish") {
        definitions.push(Box::new(
            crate::definitions::mqtt_publish::MqttPublishDefinition::from_config(config.clone())?,
        ));
    };
    #[cfg(feature = "rumqttc")]
    if let Some(config) = config.get("mqtt_subscribe") {
        definitions.push(Box::new(
            crate::definitions::mqtt_subscribe::MqttSubscribeDefinition::from_config(
                config.clone(),
            )?,
        ));
    };
    #[cfg(feature = "rumqttc")]
    if let Some(config) = config.get("mqtt_unsubscribe") {
        definitions.push(Box::new(
            crate::definitions::mqtt_unsubscribe::MqttUnsubscribeDefinition::from_config(
                config.clone(),
            )?,
        ));
    };

    if let Some(config) = config.get("print") {
        definitions.push(Box::new(
            crate::definitions::print::PrintDefinition::from_config(config.clone())?,
        ));
    };

    if let Some(config) = config.get("command") {
        definitions.push(Box::new(
            crate::definitions::command::CommandDefinition::from_config(config.clone())?,
        ));
    };
    #[cfg(feature = "reqwest")]
    if let Some(config) = config.get("api_call") {
        definitions.push(Box::new(
            crate::definitions::api_call::ApiCallDefinition::from_config(config.clone())?,
        ));
    };

    #[cfg(feature = "tiny_http")]
    if let Some(config) = config.get("api_listen") {
        definitions.push(Box::new(
            crate::definitions::api_listen::ApiListenDefinition::from_config(config.clone())?,
        ));
    };

    if let Some(config) = config.get("file_read") {
        definitions.push(Box::new(
            crate::definitions::file_read::FileReadDefinition::from_config(config.clone())?,
        ));
    };
    if let Some(config) = config.get("file_write") {
        definitions.push(Box::new(
            crate::definitions::file_write::FileWriteDefinition::from_config(config.clone())?,
        ));
    };

    #[cfg(feature = "notify")]
    if let Some(config) = config.get("file_watch") {
        definitions.push(Box::new(
            crate::definitions::file_watch::FileWatchDefinition::from_config(config.clone())?,
        ));
    };
    #[cfg(feature = "notify")]
    if let Some(config) = config.get("file_changed") {
        definitions.push(Box::new(
            crate::definitions::file_changed::FileChangedDefinition::from_config(config.clone())?,
        ));
    };
    #[cfg(all(unix, feature = "evdev"))]
    if let Some(config) = config.get("scan_code") {
        definitions.push(Box::new(
            crate::definitions::scan_code::ScanCodeDefinition::from_config(config.clone())?,
        ));
    };
    if let Some(config) = config.get("period") {
        definitions.push(Box::new(
            crate::definitions::period::PeriodDefinition::from_config(config.clone())?,
        ));
    };
    Ok(definitions)
}
