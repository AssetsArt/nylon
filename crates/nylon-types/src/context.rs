use std::collections::HashMap;

use pingora::lb::Backend;

#[derive(Debug, Clone)]
pub struct NylonContext {
    pub headers: HashMap<String, String>,
    pub backend: Backend,
    pub client_ip: String,
    pub request_id: Option<String>,
}
