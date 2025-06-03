use crate::fbs::http_context_generated::nylon_http_context::{
    KeyValue, KeyValueArgs, NylonHttpContext, NylonHttpContextArgs, NylonHttpRequest,
    NylonHttpRequestArgs, NylonHttpResponse, NylonHttpResponseArgs,
};
use nylon_error::NylonError;
use nylon_types::context::NylonContext;
use pingora::proxy::Session;
use std::collections::HashMap;

pub async fn build_http_context(
    session: &mut Session,
    params: Option<HashMap<String, String>>,
    _ctx: &mut NylonContext,
) -> Result<Vec<u8>, NylonError> {
    let mut fbs = flatbuffers::FlatBufferBuilder::new();
    // params
    let params_vec = params
        .iter()
        .flatten()
        .map(|(k, v)| {
            let key = fbs.create_string(k.as_str());
            let value = fbs.create_string(v.as_str());
            KeyValue::create(
                &mut fbs,
                &KeyValueArgs {
                    key: Some(key),
                    value: Some(value),
                },
            )
        })
        .collect::<Vec<_>>();
    let params_vec = fbs.create_vector(&params_vec);
    let body = session.read_request_body().await.unwrap_or_default();
    let request: NylonHttpRequestArgs;
    if let Some(v2) = session.as_http2() {
        let method = v2.req_header().method.as_str();
        let uri = &v2.req_header().uri;
        let path = uri.path();
        let query = uri.query().unwrap_or_default();
        let headers = v2.req_header().headers.clone();
        let headers_vec = headers
            .iter()
            .map(|(k, v)| {
                let key = fbs.create_string(k.as_str());
                let value = fbs.create_string(v.to_str().unwrap());
                KeyValue::create(
                    &mut fbs,
                    &KeyValueArgs {
                        key: Some(key),
                        value: Some(value),
                    },
                )
            })
            .collect::<Vec<_>>();
        let headers_vec = fbs.create_vector(&headers_vec);
        let body_vec = fbs.create_vector(&body.unwrap_or_default());
        request = NylonHttpRequestArgs {
            method: Some(fbs.create_string(method)),
            path: Some(fbs.create_string(path)),
            query: Some(fbs.create_string(query)),
            headers: Some(headers_vec),
            body: Some(body_vec),
            params: Some(params_vec),
        };
    } else {
        let method = session.req_header().method.as_str();
        let uri = &session.req_header().uri;
        let path = uri.path();
        let query = uri.query().unwrap_or_default();
        let headers = session.req_header().headers.clone();
        let headers_vec = headers
            .iter()
            .map(|(k, v)| {
                let key = fbs.create_string(k.as_str());
                let value = fbs.create_string(v.to_str().unwrap());
                KeyValue::create(
                    &mut fbs,
                    &KeyValueArgs {
                        key: Some(key),
                        value: Some(value),
                    },
                )
            })
            .collect::<Vec<_>>();
        let headers_vec = fbs.create_vector(&headers_vec);
        let body_vec = fbs.create_vector(&body.unwrap_or_default());
        request = NylonHttpRequestArgs {
            method: Some(fbs.create_string(method)),
            path: Some(fbs.create_string(path)),
            query: Some(fbs.create_string(query)),
            headers: Some(headers_vec),
            body: Some(body_vec),
            params: Some(params_vec),
        };
    }
    let req_offset = NylonHttpRequest::create(&mut fbs, &request);
    let response = &NylonHttpResponseArgs {
        status: 200,
        headers: None,
        body: None,
    };
    let resp_offset = NylonHttpResponse::create(&mut fbs, response);
    let dispatcher_args = &NylonHttpContextArgs {
        request: Some(req_offset),
        response: Some(resp_offset),
    };
    let dispatcher = NylonHttpContext::create(&mut fbs, dispatcher_args);
    fbs.finish(dispatcher, None);
    let dispatcher_data = fbs.finished_data();
    Ok(dispatcher_data.to_vec())
}
