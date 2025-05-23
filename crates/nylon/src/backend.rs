use nylon_error::NylonError;
use nylon_store::lb_backends::{BackendType, HttpService};
use nylon_types::context::NylonContext;
use pingora::{lb::Backend, proxy::Session};

pub fn selection(
    service: &HttpService,
    session: &mut Session,
    ctx: &mut NylonContext,
) -> Result<Backend, NylonError> {
    let mut selection_key = ctx.client_ip.clone();
    if let Some(header_value) = session.req_header().headers.get("x-forwarded-for") {
        let value = header_value.to_str().unwrap_or_default();
        selection_key.push_str(value);
    }
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
