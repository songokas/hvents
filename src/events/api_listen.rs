use std::sync::{Arc, Mutex};

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use crate::config::{Headers, PoolId};

use super::{
    api_call::{RequestContent, RequestMethod, ResponseContent},
    ReferencingEvent,
};

pub type HttpQueue = Arc<Mutex<IndexSet<ReferencingEvent>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiListenEvent {
    pub path: String,
    #[serde(default)]
    pub headers: Headers,
    pub template: Option<String>,
    #[serde(default)]
    pub method: RequestMethod,
    #[serde(default)]
    pub request_content: RequestContent,
    #[serde(default)]
    pub response_content: ResponseContent,
    #[serde(default)]
    pub action: ApiListenAction,
    #[serde(default)]
    pub pool_id: PoolId,
}

impl ApiListenEvent {
    pub fn matches(&self, url: &str, method: &str) -> bool {
        url.starts_with(&self.path)
            && self.method.to_string().to_uppercase() == method.to_uppercase()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ApiListenAction {
    #[default]
    Start,
    Stop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_listen_matches() {
        let data = [
            (
                "match url exactly",
                create_listen_event("/clients/1", Default::default()),
                "/clients/1",
                "get",
                true,
            ),
            (
                "match url by prefix",
                create_listen_event("/clients/", Default::default()),
                "/clients/1",
                "get",
                true,
            ),
            (
                "match method exactly",
                create_listen_event("/clients/1", RequestMethod::Post),
                "/clients/1",
                "post",
                true,
            ),
            (
                "different url",
                create_listen_event("/clients/1", RequestMethod::Post),
                "/clients",
                "post",
                false,
            ),
            (
                "different methods",
                create_listen_event("/clients/1", RequestMethod::Post),
                "/clients",
                "get",
                false,
            ),
        ];
        for (test_name, event, uri, method, expected) in data {
            assert_eq!(event.matches(uri, method), expected, "{test_name}");
        }
    }

    fn create_listen_event(uri: &str, request_method: RequestMethod) -> ApiListenEvent {
        ApiListenEvent {
            path: uri.to_string(),
            headers: Default::default(),
            template: Default::default(),
            method: request_method,
            request_content: Default::default(),
            response_content: Default::default(),
            action: Default::default(),
            pool_id: Default::default(),
        }
    }
}
