use pingora::lb::Backend;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NylonContext {
    pub headers: HashMap<String, String>,
    pub backend: Backend,
    pub client_ip: String,
}
