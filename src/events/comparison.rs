use core::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::EnumString;

use crate::renderer::TemplateData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperation {
    Equal,
    NotEqual,
    Lt,
    Gt,
}

impl FromStr for ComparisonOperation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "is equal to" => ComparisonOperation::Equal,
            "is not equal to" => ComparisonOperation::NotEqual,
            "is above" | "is greater than" => ComparisonOperation::Gt,
            "is below" | "is lower than" => ComparisonOperation::Lt,
            s => return Err(format!("Unknown comparison {s}")),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DataToCompare {
    Data,
    Metadata,
    State,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonEvent {
    pub data_to_compare: DataToCompare,
    pub key: String,
    pub operation: ComparisonOperation,
    pub value: String,
}

impl ComparisonEvent {
    pub fn matches(&self, template_data: &TemplateData) -> bool {
        let v = match self.data_to_compare {
            DataToCompare::Data => template_data.data.get(&self.key),
            DataToCompare::Metadata => template_data.metadata.get(&self.key),
            DataToCompare::State => template_data.state.get(&self.key).map(|s| s.as_str()),
        };
        let Some(v) = v else {
            return false;
        };

        match self.operation {
            ComparisonOperation::Equal => v == &self.value,
            ComparisonOperation::NotEqual => v != &self.value,
            ComparisonOperation::Lt => v < self.value.as_str(),
            ComparisonOperation::Gt => v > self.value.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        let json = r#"{
            "data_to_compare": "state",
            "key": "test",
            "operation": "lt",
            "value": "15"
        }"#;

        serde_json::from_str::<ComparisonEvent>(json).unwrap();
    }
}
