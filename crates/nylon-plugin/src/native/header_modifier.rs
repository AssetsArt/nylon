use nylon_error::NylonError;
use nylon_types::{
    context::NylonContext,
    template::{Expr, apply_payload_ast},
};
use pingora::proxy::Session;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
struct Payload {
    remove: Option<Vec<String>>,
    set: Option<Vec<Header>>,
}

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
    let payload = match payload.as_ref() {
        Some(payload) => {
            let mut payload = payload.clone();
            if let Some(payload_ast) = payload_ast {
                apply_payload_ast(&mut payload, payload_ast, ctx);
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
        let headers = session.req_header_mut();
        for header in set {
            let _ = headers.remove_header(&header.name);
            let name = header.name.to_ascii_uppercase();
            let _ = headers.append_header(name, &header.value);
        }
    }
    if let Some(remove) = payload.remove {
        let headers = session.req_header_mut();
        for header in remove {
            let _ = headers.remove_header(&header);
        }
    }
    Ok(())
}
