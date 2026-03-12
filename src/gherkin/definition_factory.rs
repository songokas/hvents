use anyhow::Result;

use crate::{
    config::DefinitionConfig,
    definitions::{
        api_call::ApiCallDefinition, api_listen::ApiListenDefinition, command::CommandDefinition,
        data_comparison::DataComparisonDefinition, file_changed::FileChangedDefinition,
        file_read::FileReadDefinition, file_watch::FileWatchDefinition,
        file_write::FileWriteDefinition, mqtt_publish::MqttPublishDefinition,
        mqtt_subscribe::MqttSubscribeDefinition, mqtt_unsubscribe::MqttUnsubscribeDefinition,
        once::OnceDefinition, period::PeriodDefinition, print::PrintDefinition,
        repeat::RepeatDefinition, scan_code::ScanCodeDefinition,
        state_modification::StateModificationDefinition,
    },
    gherkin::event_factory::EventFactory,
};

pub fn create_definitions(config: DefinitionConfig) -> Result<Vec<Box<dyn EventFactory>>> {
    let mut definitions: Vec<Box<dyn EventFactory>> = vec![];
    if let Some(config) = config.get("repeat") {
        definitions.push(Box::new(RepeatDefinition::from_config(config.clone())?));
    };
    if let Some(config) = config.get("once") {
        definitions.push(Box::new(OnceDefinition::from_config(config.clone())?));
    };

    if let Some(config) = config.get("comparison") {
        definitions.push(Box::new(DataComparisonDefinition::from_config(
            config.clone(),
        )?));
    };

    if let Some(config) = config.get("state_modifications") {
        definitions.push(Box::new(StateModificationDefinition::from_config(
            config.clone(),
        )?));
    };

    if let Some(config) = config.get("mqtt_publish") {
        definitions.push(Box::new(MqttPublishDefinition::from_config(
            config.clone(),
        )?));
    };
    if let Some(config) = config.get("mqtt_subscribe") {
        definitions.push(Box::new(MqttSubscribeDefinition::from_config(
            config.clone(),
        )?));
    };
    if let Some(config) = config.get("mqtt_unsubscribe") {
        definitions.push(Box::new(MqttUnsubscribeDefinition::from_config(
            config.clone(),
        )?));
    };

    if let Some(config) = config.get("print") {
        definitions.push(Box::new(PrintDefinition::from_config(config.clone())?));
    };

    if let Some(config) = config.get("command") {
        definitions.push(Box::new(CommandDefinition::from_config(config.clone())?));
    };

    if let Some(config) = config.get("api_call") {
        definitions.push(Box::new(ApiCallDefinition::from_config(config.clone())?));
    };

    if let Some(config) = config.get("api_listen") {
        definitions.push(Box::new(ApiListenDefinition::from_config(config.clone())?));
    };

    if let Some(config) = config.get("file_read") {
        definitions.push(Box::new(FileReadDefinition::from_config(config.clone())?));
    };
    if let Some(config) = config.get("file_write") {
        definitions.push(Box::new(FileWriteDefinition::from_config(config.clone())?));
    };

    if let Some(config) = config.get("file_watch") {
        definitions.push(Box::new(FileWatchDefinition::from_config(config.clone())?));
    };
    if let Some(config) = config.get("file_changed") {
        definitions.push(Box::new(FileChangedDefinition::from_config(
            config.clone(),
        )?));
    };
    if let Some(config) = config.get("scan_code") {
        definitions.push(Box::new(ScanCodeDefinition::from_config(config.clone())?));
    };
    if let Some(config) = config.get("period") {
        definitions.push(Box::new(PeriodDefinition::from_config(config.clone())?));
    };
    Ok(definitions)
}
