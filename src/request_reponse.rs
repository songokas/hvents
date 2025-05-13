use core::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RequestMethod {
    Put,
    Post,
    #[default]
    Get,
    Delete,
}

impl Display for RequestMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestMethod::Put => write!(f, "PUT"),
            RequestMethod::Post => write!(f, "POST"),
            RequestMethod::Get => write!(f, "GET"),
            RequestMethod::Delete => write!(f, "DELETE"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RequestContent {
    Json,
    Text,
    #[default]
    Bytes,
}

impl Display for RequestContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestContent::Json => write!(f, "json"),
            RequestContent::Text => write!(f, "text"),
            RequestContent::Bytes => write!(f, "bytes"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ResponseContent {
    Json,
    #[default]
    Text,
    Bytes,
}

impl Display for ResponseContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseContent::Json => write!(f, "json"),
            ResponseContent::Text => write!(f, "text"),
            ResponseContent::Bytes => write!(f, "bytes"),
        }
    }
}
