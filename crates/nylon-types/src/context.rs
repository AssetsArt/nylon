use pingora::lb::Backend;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NylonContext {
    pub headers: HashMap<String, String>,
    pub backend: Backend,
    pub client_ip: String,
}

impl Default for NylonContext {
    fn default() -> Self {
        Self {
            headers: HashMap::new(),
            backend: Backend::new("127.0.0.1:80").expect("Unable to create backend"),
            client_ip: "127.0.0.1".to_string(),
        }
    }
}
