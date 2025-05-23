use pingora::lb::Backend;

#[derive(Debug, Clone)]
pub struct NylonContext {
    pub backend: Backend,
    pub client_ip: String,
}
