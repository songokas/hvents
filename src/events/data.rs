use core::str::from_utf8;
use std::{borrow::Cow, io::Read};

use serde::{de, Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(untagged)]
pub enum Data {
    String(String),
    #[serde(deserialize_with = "any_value")]
    Json(Value),
    Bytes(Vec<u8>),
    #[default]
    Empty,
}

impl From<&str> for Data {
    fn from(value: &str) -> Self {
        Data::String(value.to_string())
    }
}

impl From<&[u8]> for Data {
    fn from(value: &[u8]) -> Self {
        Data::Bytes(value.to_vec())
    }
}

impl From<Value> for Data {
    fn from(value: Value) -> Self {
        Data::Json(value)
    }
}

impl Data {
    pub fn from_reader(mut reader: impl Read, data_type: DataType) -> anyhow::Result<Self> {
        Ok(match data_type {
            DataType::String => {
                let mut s = String::default();
                reader.read_to_string(&mut s)?;
                Data::String(s)
            }
            DataType::Bytes => {
                let mut buf = Vec::default();
                reader.read_to_end(&mut buf)?;
                Data::Bytes(buf)
            }
            DataType::Json => {
                let value: Value = serde_json::from_reader(reader)?;
                Data::Json(value)
            }
        })
    }

    pub fn as_bytes(&self) -> anyhow::Result<Cow<[u8]>> {
        Ok(match self {
            Data::Json(j) => serde_json::to_vec(j)?.into(),
            Data::String(s) => s.as_bytes().into(),
            Data::Bytes(b) => b.into(),
            Data::Empty => [].as_ref().into(),
        })
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(match self {
            Data::Json(j) => serde_json::to_vec(j)?,
            Data::String(s) => s.as_bytes().to_vec(),
            Data::Bytes(b) => b.clone(),
            Data::Empty => Vec::default(),
        })
    }

    pub fn merge(&mut self, data: Data) {
        match (self, data) {
            (Data::Json(a), Data::Json(b)) => merge_json_value_recursive(a, b),
            (Data::String(a), Data::String(b)) => a.push_str(&b),
            (Data::Bytes(a), Data::String(b)) => a.extend_from_slice(b.as_bytes()),
            (Data::Bytes(a), Data::Bytes(b)) => a.extend(b),
            (_, Data::Empty) => (),
            (s, d) => *s = d,
        }
    }

    pub fn try_merge_bytes(&mut self, bytes: &[u8]) {
        let data: Data = if let Ok(v) = serde_json::from_slice(bytes) {
            Data::Json(v)
        } else if let Ok(v) = from_utf8(bytes) {
            Data::String(v.to_string())
        } else {
            Data::Bytes(bytes.to_vec())
        };
        self.merge(data);
    }
}

impl PartialEq<Value> for Data {
    fn eq(&self, other: &Value) -> bool {
        match self {
            Data::Json(v) => v == other,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Copy)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    #[default]
    String,
    Bytes,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Metadata(Value);

impl Metadata {
    pub fn merge(&mut self, metadata: Metadata) {
        merge_json_value_recursive(&mut self.0, metadata.0)
    }
}

impl From<Value> for Metadata {
    fn from(value: Value) -> Self {
        Metadata(value)
    }
}

fn merge_json_value_recursive(a: &mut Value, b: Value) {
    if let Value::Object(a) = a {
        if let Value::Object(b) = b {
            for (k, v) in b {
                if v.is_null() {
                    a.remove(&k);
                } else {
                    merge_json_value_recursive(a.entry(k).or_insert(Value::Null), v);
                }
            }

            return;
        }
    }
    *a = b;
}

pub fn any_value<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum AnyValue {
        Json(Value),
        Yaml(serde_yaml::Value),
    }
    let s: AnyValue = de::Deserialize::deserialize(deserializer)?;
    match s {
        AnyValue::Json(t) => Ok(t),
        AnyValue::Yaml(t) => serde_json::to_value(t).map_err(de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_data_json_serialization() {
        let json = r#"{"a":"b"}"#;
        let value: Value = serde_json::from_str(json).unwrap();
        let data: Data = serde_json::from_str(json).unwrap();
        assert_eq!(data, Data::Json(value))
    }

    #[test]
    fn test_data_string_serialization() {
        let s = r#""simple string""#;
        let data: Data = serde_json::from_str(s).unwrap();
        assert_eq!(data, Data::String("simple string".to_string()));
    }

    #[test]
    fn test_merge_bytes_with_string() {
        // bytes append_string
        let mut data: Data = b"1".as_ref().into();
        data.merge("2".into());
        assert_eq!(data.as_bytes().unwrap(), b"12".as_ref());
        assert!(matches!(data, Data::Bytes(_)));

        let mut data: Data = "1".into();
        data.merge(b"2".as_ref().into());
        assert_eq!(data.as_bytes().unwrap(), b"2".as_ref());
        assert!(matches!(data, Data::Bytes(_)));
    }

    #[test]
    fn test_overwrite_string_with_bytes() {
        let mut data: Data = "1".into();
        data.merge(b"2".as_ref().into());
        assert_eq!(data.as_bytes().unwrap(), b"2".as_ref());
        assert!(matches!(data, Data::Bytes(_)));
    }

    #[test]
    fn test_merge_json() {
        let mut data: Data = json!({"a":"1"}).into();
        data.merge(json!({"b":"2"}).into());
        dbg!(&data);
        assert_eq!(data.as_bytes().unwrap(), br#"{"a":"1","b":"2"}"#.as_ref());
        assert!(matches!(data, Data::Json(_)));
    }

    #[test]
    fn test_overwrite_empty() {
        let json_data: Data = json!({"a":"1"}).into();
        let string_data: Data = "1".into();
        let byte_data: Data = b"1".as_ref().into();

        let mut data = Data::Empty;
        data.merge(json_data);
        assert!(matches!(data, Data::Json(_)));
        let mut data = Data::Empty;
        data.merge(string_data);
        assert!(matches!(data, Data::String(_)));
        let mut data = Data::Empty;
        data.merge(byte_data);
        assert!(matches!(data, Data::Bytes(_)));
    }

    #[test]
    fn test_skip_overwrite_if_empty() {
        let mut json_data: Data = json!({"a":"1"}).into();
        let mut string_data: Data = "1".into();
        let mut byte_data: Data = b"1".as_ref().into();

        json_data.merge(Data::Empty);
        assert!(matches!(json_data, Data::Json(_)));
        string_data.merge(Data::Empty);
        assert!(matches!(string_data, Data::String(_)));
        byte_data.merge(Data::Empty);
        assert!(matches!(byte_data, Data::Bytes(_)));
    }
}
