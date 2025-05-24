use nylon_error::NylonError;
use nylon_types::context::NylonContext;
use pingora::proxy::Session;
use serde::Deserialize;
use serde_json::Value;

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
    _ctx: &mut NylonContext,
    session: &mut Session,
    payload: &Option<Value>,
) -> Result<(), NylonError> {
    let payload = match payload {
        Some(payload) => serde_json::from_value::<Payload>(payload.clone())
            .map_err(|e| NylonError::ConfigError(e.to_string()))?,
        None => Payload {
            remove: None,
            set: None,
        },
    };

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
