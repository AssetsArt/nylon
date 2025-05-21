use nylon_error::NylonError;
use nylon_store::lb_backends::{BackendType, HttpService};
use pingora::lb::Backend;

pub fn selection(selection_key: &str, service: &HttpService) -> Result<Backend, NylonError> {
    match &service.backend_type {
        BackendType::RoundRobin(lb) => lb.select(selection_key.as_bytes(), 256),
        BackendType::Weighted(lb) => lb.select(selection_key.as_bytes(), 256),
        BackendType::Consistent(lb) => lb.select(selection_key.as_bytes(), 256),
        BackendType::Random(lb) => lb.select(selection_key.as_bytes(), 256),
    }
    .ok_or(NylonError::HttpException(
        500,
        "INTERNAL_SERVER_ERROR",
        "No backend found",
    ))
}
