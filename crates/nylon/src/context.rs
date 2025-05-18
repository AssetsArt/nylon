use pingora::lb::Backend;
use std::collections::HashMap;

pub struct NylonContext {
    pub backend: Backend,
    pub variables: HashMap<String, String>,
}

impl NylonContext {
    pub fn new() -> Self {
        Self {
            backend: Backend::new("127.0.0.1:80").expect("Unable to create backend"),
            variables: HashMap::new(),
        }
    }
}
