use serde::{de, Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCodeReadEvent(#[serde(deserialize_with = "deserialize_code")] i32);

impl ScanCodeReadEvent {
    pub fn new(code: i32) -> Self {
        Self(code)
    }

    pub fn matches(&self, code: i32) -> bool {
        self.0 == code
    }
}

fn deserialize_code<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: de::Deserializer<'de>,
{
    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum CodeTypes {
        Number(i32),
        String(String),
    }
    let s: CodeTypes = de::Deserialize::deserialize(deserializer)?;
    Ok(match s {
        CodeTypes::Number(v) => v,
        CodeTypes::String(v) => {
            let mut v = hex::decode(v.trim_start_matches("0x")).map_err(de::Error::custom)?;
            while v.len() < 4 {
                v.push(0);
            }
            let bytes: [u8; 4] = v.as_slice().try_into().map_err(de::Error::custom)?;
            i32::from_ne_bytes(bytes)
        }
    })
}
