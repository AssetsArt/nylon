pub mod context;
pub mod plugins;
pub mod proxy;
pub mod route;
pub mod services;
pub mod template;
pub mod tls;

/// Nylon runtime server instance
#[derive(Debug, Clone)]
pub struct NylonRuntime {}
