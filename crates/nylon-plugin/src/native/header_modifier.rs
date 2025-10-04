use nylon_error::NylonError;
use nylon_types::{
    context::NylonContext,
    template::{Expr, apply_payload_ast},
};
use pingora::proxy::Session;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Payload structure for header modification
#[derive(Debug, Deserialize, Clone)]
struct Payload {
    remove: Option<Vec<String>>,
    set: Option<Vec<Header>>,
}

/// Header structure for setting headers
#[derive(Debug, Deserialize, Clone)]
struct Header {
    name: String,
    value: String,
}

pub fn request(
    ctx: &mut NylonContext,
    session: &mut Session,
    payload: &Option<Value>,
    payload_ast: &Option<HashMap<String, Vec<Expr>>>,
) -> Result<(), NylonError> {
    let headers = session.req_header_mut();
    let payload = match payload.as_ref() {
        Some(payload) => {
            let mut payload = payload.clone();
            if let Some(payload_ast) = payload_ast {
                apply_payload_ast(&mut payload, payload_ast, headers, ctx);
            }
            serde_json::from_value::<Payload>(payload.clone())
                .map_err(|e| NylonError::ConfigError(e.to_string()))?
        }
        None => Payload {
            remove: None,
            set: None,
        },
    };
    // println!("payload: {:#?}", payload);
    if let Some(set) = payload.set {
        for header in set {
            let _ = headers.remove_header(&header.name);
            let name = header.name.to_ascii_lowercase();
            let _ = headers.append_header(name.clone(), &header.value);
        }
    }
    if let Some(remove) = payload.remove {
        for header in remove {
            let _ = headers.remove_header(&header.to_ascii_lowercase());
        }
    }
    Ok(())
}

pub fn response(
    ctx: &mut NylonContext,
    session: &mut Session,
    payload: &Option<Value>,
    payload_ast: &Option<HashMap<String, Vec<Expr>>>,
) -> Result<(), NylonError> {
    let headers = session.req_header();
    let payload = match payload.as_ref() {
        Some(payload) => {
            let mut payload = payload.clone();
            if let Some(payload_ast) = payload_ast {
                apply_payload_ast(&mut payload, payload_ast, headers, ctx);
            }
            serde_json::from_value::<Payload>(payload.clone())
                .map_err(|e| NylonError::ConfigError(e.to_string()))?
        }
        None => Payload {
            remove: None,
            set: None,
        },
    };
    if let Some(set) = payload.set {
        let mut map = ctx.add_response_header.write().expect("lock");
        for header in set {
            let _ = map.insert(header.name, header.value);
        }
    }
    if let Some(remove) = payload.remove {
        let mut vec = ctx.remove_response_header.write().expect("lock");
        for header in remove {
            vec.push(header);
        }
    }
    Ok(())
}
