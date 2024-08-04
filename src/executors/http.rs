use std::sync::mpsc::Sender;

use anyhow::anyhow;
use indexmap::IndexSet;
use log::{debug, error, warn};
use serde_json::Value;
use tiny_http::{Header, Method, Request, Response, Server};

use crate::{
    config::Headers,
    events::{
        api_call::{RequestContent, ResponseContent},
        api_listen::HttpQueue,
        data::Data,
        EventType, Events, ReferencingEvent,
    },
    renderer::load_handlebars,
};

pub fn http_executor(
    http_queue: HttpQueue,
    listen: &str,
    events: &Events,
    queue_tx: Sender<ReferencingEvent>,
) -> anyhow::Result<()> {
    let server = Server::http(listen)
        .map_err(|e| anyhow!("Http server failed to listen to {listen} {e}"))?;
    let handlebars = load_handlebars();

    for mut request in server.incoming_requests() {
        debug!(
            "Incoming request method: {:?}, url: {:?}, headers: {:?}",
            request.method(),
            request.url(),
            request.headers()
        );

        let response = match handle_incoming(
            events,
            &http_queue.lock().expect("http queue locked"),
            &handlebars,
            &mut request,
        ) {
            Some(output) => {
                if let Some(e) = output.event {
                    queue_tx.send(e)?;
                }
                let mut response = Response::from_data(output.data);
                for (k, v) in output.headers {
                    match Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                        Ok(h) => response.add_header(h),
                        Err(_) => warn!("Failed to add header {k} {v}"),
                    };
                }
                response
            }
            None => Response::from_string("Not Found").with_status_code(404),
        };

        match request.respond(response) {
            Ok(_) => debug!("Http response sent"),
            Err(e) => warn!("Http response failed {e}"),
        };
    }
    Ok(())
}

fn handle_incoming(
    events: &Events,
    http_events: &IndexSet<ReferencingEvent>,
    handlebars: &handlebars::Handlebars,
    request: &mut Request,
) -> Option<ResponseData> {
    let (ref_event, listen_event) =
        http_events
            .iter()
            .find_map(|ref_event| match &ref_event.event_type {
                EventType::ApiListen(e) if e.matches(request.url(), request.method().as_str()) => {
                    Some((ref_event, e))
                }
                _ => None,
            })?;

    debug!(
        "Http found event {} next event {:?} request content {} response content {}",
        ref_event.name,
        ref_event.next_event,
        listen_event.request_content,
        listen_event.response_content
    );

    let request_content: Option<Data> = match (request.method(), &listen_event.request_content) {
        (Method::Post | Method::Put, RequestContent::Json) => {
            match serde_json::from_reader::<_, Value>(request.as_reader()) {
                Ok(v) => Data::Json(v).into(),
                Err(e) => {
                    error!("Failed to read request payload {e}");
                    return None;
                }
            }
        }
        (Method::Post | Method::Put, RequestContent::Text) => {
            let mut content = String::new();
            if let Err(e) = request.as_reader().read_to_string(&mut content) {
                error!("Failed to read request payload {e}");
                return None;
            }
            Data::String(content).into()
        }
        (Method::Post | Method::Put, RequestContent::Bytes) => {
            let mut content = Vec::default();
            if let Err(e) = request.as_reader().read_to_end(&mut content) {
                error!("Failed to read request payload {e}");
                return None;
            }
            Data::Bytes(content).into()
        }
        _ => None,
    };

    let mut headers = listen_event.headers.clone();

    let mut data = Data::default();
    if let Some(c) = request_content.clone() {
        data.merge(c);
    }
    data.merge(ref_event.data.clone());

    let template_response = if let Some(t) = &listen_event.template {
        let mut content = Vec::default();
        if let Err(e) = handlebars.render_template_to_write(t, &data, &mut content) {
            error!("Failed to render template {e} event={}", ref_event.name);
            return None;
        }
        content.into()
    } else {
        None
    };

    let response_content = match (&listen_event.response_content, template_response) {
        (ResponseContent::Json, None) => match serde_json::to_vec(&ref_event.data) {
            Ok(s) => {
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                s
            }
            Err(e) => {
                error!("Failed to serialize json {e}");
                return None;
            }
        },
        (ResponseContent::Json, Some(t)) => {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
            t
        }

        (ResponseContent::Text, None) => match &ref_event.data {
            Data::String(s) => s.as_bytes().to_vec(),
            _ => {
                warn!("Responding with OK unknown data");
                "OK".as_bytes().to_vec()
            }
        },
        (ResponseContent::Text, Some(t)) => t,
        (ResponseContent::Bytes, _) => match ref_event.data.to_bytes() {
            Ok(b) => b,
            Err(e) => {
                warn!("Responding with OK unknown data {e}");
                "OK".as_bytes().to_vec()
            }
        },
    };

    if let Some(mut event) = ref_event
        .next_event
        .as_ref()
        .and_then(|e| events.get_event_by_name(e))
    {
        event.data.merge(data);
        ResponseData {
            event: event.into(),
            data: response_content,
            headers,
        }
        .into()
    } else {
        debug!("Received event {} without further handler", ref_event.name);
        ResponseData {
            event: None,
            data: response_content,
            headers,
        }
        .into()
    }
}

struct ResponseData {
    event: Option<ReferencingEvent>,
    data: Vec<u8>,
    headers: Headers,
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc::channel, thread::spawn, time::Duration};

    use serde_json::json;

    use crate::events::{
        api_call::RequestMethod,
        api_listen::{ApiListenEvent, HttpQueue},
        time::TimeEvent,
    };

    use super::*;

    #[test]
    fn test_executor() {
        let (queue_tx, queue_rx) = channel();

        let events = [
            create_time_event("test1", json!({ "test1": "text" })),
            create_time_event("test2", Default::default()),
            create_time_event("test3", json!({ "test3": "text" })),
            create_time_event("test4", json!({ "test4": "text" })),
        ];

        spawn(move || {
            let queue = HttpQueue::default();
            queue.lock().unwrap().insert(create_listen_event(
                "listen1",
                Some("test1".to_string()),
                json!({ "listen1": "text" }),
                "/clients/listen1",
                RequestMethod::Get,
                None,
            ));
            queue.lock().unwrap().insert(create_listen_event(
                "listen2",
                Some("test1".to_string()),
                json!({ "listen2": "currently" }),
                "/clients",
                RequestMethod::Post,
                r#"{{listen2}} {{time}}"#.to_string().into(),
            ));
            let events = Events::new(events.into_iter().collect());
            http_executor(queue, "127.0.0.1:13333", &events, queue_tx.clone()).unwrap();
        });

        let body = reqwest::blocking::get("http://127.0.0.1:13333/clients/listen1")
            .unwrap()
            .text()
            .unwrap();

        assert_eq!(body, r#"{"listen1":"text"}"#);

        let body = reqwest::blocking::Client::new()
            .post("http://127.0.0.1:13333/clients/listen1")
            .body(r#"{"time":"2024-01-01"}"#)
            .send()
            .unwrap()
            .text()
            .unwrap();

        assert_eq!(body, r#"currently 2024-01-01"#);

        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test1");
        assert_eq!(event.data, json!({ "test1": "text", "listen1": "text" }));
        let event = queue_rx.recv_timeout(Duration::from_millis(200)).unwrap();
        assert_eq!(event.name, "test1");
        assert_eq!(
            event.data,
            json!({ "test1": "text", "listen2": "currently", "time":"2024-01-01" })
        );
    }

    fn create_time_event(name: &str, data: Value) -> ReferencingEvent {
        ReferencingEvent {
            event_type: EventType::Time(TimeEvent {
                execute_time: None,
                execute_period: None,
            }),
            next_event: None,
            data: Data::Json(data),
            next_event_template: None,
            name: name.to_string(),
        }
    }

    fn create_listen_event(
        name: &str,
        next_event: Option<String>,
        data: Value,
        uri: &str,
        request_method: RequestMethod,
        template: Option<String>,
    ) -> ReferencingEvent {
        ReferencingEvent {
            event_type: EventType::ApiListen(ApiListenEvent {
                path: uri.to_string(),
                headers: Default::default(),
                template,
                method: request_method,
                request_content: RequestContent::Json,
                response_content: ResponseContent::Json,
                action: Default::default(),
                pool_id: Default::default(),
            }),
            next_event,
            data: Data::Json(data),
            next_event_template: None,
            name: name.to_string(),
        }
    }
}
