use core::str::from_utf8;
use std::{borrow::Cow, io::Read};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(untagged)]
pub enum Data {
    String(String),
    Json(Value),
    Bytes(Vec<u8>),
    #[default]
    Empty,
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

    pub fn as_bytes(&self) -> Cow<[u8]> {
        match self {
            Data::Json(j) => serde_json::to_vec(j).expect("valid json").into(),
            Data::String(s) => s.as_bytes().into(),
            Data::Bytes(b) => b.into(),
            Data::Empty => [].as_ref().into(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Data::Json(j) => serde_json::to_vec(j).expect("valid json"),
            Data::String(s) => s.as_bytes().to_vec(),
            Data::Bytes(b) => b.clone(),
            Data::Empty => Vec::default(),
        }
    }

    pub fn merge(&mut self, data: Data) {
        match (self, data) {
            (Data::Json(a), Data::Json(b)) => merge_json_value_recursive(a, b),
            (Data::Json(_), _) => (),
            (Data::String(a), Data::String(b)) => a.push_str(&b),
            (Data::String(_), _) => (),
            (Data::Bytes(_), _) => (),
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

#[cfg(test)]
mod tests {
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
}
