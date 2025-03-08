use std::sync::{Arc, Mutex};

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use tiny_http::Header;

use crate::config::{Headers, PoolId};

use super::{
    api_call::{RequestContent, RequestMethod, ResponseContent},
    ReferencingEvent,
};

pub type HttpQueue = Arc<Mutex<IndexSet<ReferencingEvent>>>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiListenEvent {
    pub path: String,
    #[serde(default)]
    pub method: RequestMethod,
    #[serde(default)]
    pub request_content: RequestContent,
    #[serde(default)]
    pub request_headers: Vec<Headers>,
    #[serde(default)]
    pub response_headers: Headers,
    pub response_body: Option<String>,
    #[serde(default)]
    pub response_content: ResponseContent,
    #[serde(default)]
    pub action: ApiListenAction,
    #[serde(default)]
    pub pool_id: PoolId,
}

impl ApiListenEvent {
    pub fn matches(&self, url: &str, method: &str, headers: &[Header]) -> bool {
        url.starts_with(&self.path)
            && self.method.to_string().to_uppercase() == method.to_uppercase()
            && (self.request_headers.is_empty() || self.matches_headers(headers))
    }

    fn matches_headers(&self, headers: &[Header]) -> bool {
        self.request_headers.iter().any(|h| {
            h.iter().all(|(expected_key, expected_value)| {
                headers.iter().any(|h| {
                    h.field.as_str() == expected_key.as_str() && h.value.as_str() == expected_value
                })
            })
        })
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
                create_listen_event("/clients/1", Default::default(), &[]),
                "/clients/1",
                "get",
                vec![("expected_header", "1")],
                true,
            ),
            (
                "match url by prefix and headers",
                create_listen_event(
                    "/clients/",
                    Default::default(),
                    &[&[("expected_header", "1")]],
                ),
                "/clients/1",
                "get",
                vec![("expected_header", "1")],
                true,
            ),
            (
                "match method exactly",
                create_listen_event("/clients/1", RequestMethod::Post, &[]),
                "/clients/1",
                "post",
                vec![],
                true,
            ),
            (
                "match any one of the headers",
                create_listen_event(
                    "/clients/",
                    Default::default(),
                    &[&[
                        ("Authorization", "Basic dGVzdDEK"),
                        ("Authorization", "Basic dGVzdDIK"),
                    ]],
                ),
                "/clients/1",
                "get",
                vec![("Authorization", "Basic dGVzdDIK")],
                true,
            ),
            (
                "different url",
                create_listen_event("/clients/1", RequestMethod::Post, &[]),
                "/clients",
                "post",
                vec![],
                false,
            ),
            (
                "different methods",
                create_listen_event("/clients/1", RequestMethod::Post, &[]),
                "/clients",
                "get",
                vec![],
                false,
            ),
            (
                "different headers",
                create_listen_event(
                    "/clients/1",
                    RequestMethod::Post,
                    &[&[("expected_header", "1")]],
                ),
                "/clients",
                "get",
                vec![],
                false,
            ),
        ];
        for (test_name, event, uri, method, headers, expected) in data {
            let headers: Vec<Header> = headers
                .iter()
                .map(|(k, v)| Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap())
                .collect();
            assert_eq!(
                event.matches(uri, method, &headers),
                expected,
                "{test_name}"
            );
        }
    }

    fn create_listen_event(
        uri: &str,
        request_method: RequestMethod,
        headers: &[&[(&str, &str)]],
    ) -> ApiListenEvent {
        ApiListenEvent {
            path: uri.to_string(),
            response_headers: Default::default(),
            response_body: Default::default(),
            method: request_method,
            request_content: Default::default(),
            response_content: Default::default(),
            action: Default::default(),
            pool_id: Default::default(),
            request_headers: headers
                .into_iter()
                .map(|a| {
                    a.into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect()
                })
                .collect(),
        }
    }
}
