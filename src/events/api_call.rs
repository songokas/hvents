use std::collections::HashMap;

use anyhow::anyhow;
use indexmap::IndexMap;
use log::debug;
use reqwest::{
    blocking::Client,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    config::PoolId,
    events::data::Metadata,
    request_reponse::{RequestContent, RequestMethod, ResponseContent},
};

use super::data::Data;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiCallEvent {
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub method: RequestMethod,
    pub request_body: Option<String>,
    #[serde(default)]
    pub request_content: RequestContent,
    #[serde(default)]
    pub response_content: ResponseContent,
    #[serde(default)]
    pub pool_id: PoolId,
}

impl ApiCallEvent {
    pub fn call_api(
        &self,
        client: &Client,
        data: &Data,
        name: &str,
    ) -> Result<(Data, Metadata), anyhow::Error> {
        let mut headers: HeaderMap = (&self.headers)
            .try_into()
            .map_err(|e| anyhow!("Invalid header specified: {e}"))?;
        if let RequestContent::Json = &self.request_content {
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        };

        debug!("Request to {} body {data:?} headers {headers:?}", self.url);
        let response = match &self.method {
            RequestMethod::Delete => client.delete(&self.url).headers(headers).send()?,
            RequestMethod::Put => client
                .put(&self.url)
                .body(data.to_bytes()?)
                .headers(headers)
                .send()?,
            RequestMethod::Post => client
                .post(&self.url)
                .body(data.to_bytes()?)
                .headers(headers)
                .send()?,
            RequestMethod::Get => client.get(&self.url).headers(headers).send()?,
        };
        debug!("Response from {} {response:?}", self.url);
        let mut meta = json!({"headers": response.headers().into_iter().filter_map(|(k, v)| Some((k.as_str(), v.to_str().ok()?))).collect::<IndexMap<&str, &str>>()});
        meta[name] = meta.clone();
        let bytes = response.bytes()?;
        let data = match &self.response_content {
            ResponseContent::Json => Data::Json(serde_json::from_slice(&bytes)?),
            ResponseContent::Text => Data::String(String::from_utf8_lossy(&bytes).to_string()),
            ResponseContent::Bytes => Data::Bytes(bytes.to_vec()),
        };
        Ok((data, meta.into()))
    }
}
