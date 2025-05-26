#![allow(clippy::type_complexity)]
use crate as store;
use nylon_error::NylonError;
use nylon_types::{
    context::Route,
    route::{MiddlewareItem, RouteConfig, PathConfig},
    services::ServiceItem,
    template::{Expr, extract_and_parse_templates, walk_json},
};
use pingora::proxy::Session;
use serde_json::Value;
use std::collections::HashMap;

fn parsed_middleware(
    middleware: Vec<MiddlewareItem>,
    to: &mut Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>,
) {
    for m in middleware {
        let mut payload_ast = HashMap::<String, Vec<Expr>>::new();
        if let Some(payload) = &m.payload {
            walk_json(payload, "".to_string(), &mut |path, val| {
                if let Some(s) = val.as_str() {
                    let ast = extract_and_parse_templates(s).unwrap_or_default();
                    if !ast.is_empty() {
                        payload_ast.insert(path, ast);
                    }
                }
            });
        }
        to.push((m, Some(payload_ast)));
    }
}

pub fn store(
    routes: Vec<&RouteConfig>,
    services: &Vec<&ServiceItem>,
    middleware_groups: &Option<HashMap<String, Vec<MiddlewareItem>>>,
) -> Result<(), NylonError> {
    let middleware_groups = middleware_groups.clone().unwrap_or_default();
    let mut store_route = HashMap::new();
    let mut globa_routes_matchit = HashMap::new();

    for route in routes {
        process_route_matcher(route, &mut store_route)?;
        let route_middleware = process_route_middleware(route, &middleware_groups)?;
        let matchit_route = create_matchit_router(route, services, &route_middleware, &middleware_groups)?;
        globa_routes_matchit.insert(route.name.clone(), matchit_route);
    }

    store::insert(store::KEY_ROUTES_MATCHIT, globa_routes_matchit);
    store::insert(store::KEY_ROUTES, store_route);
    Ok(())
}

fn process_route_matcher(route: &RouteConfig, store_route: &mut HashMap<String, String>) -> Result<(), NylonError> {
    match route.route.kind.as_str() {
        "host" => {
            for host in route.route.value.split('|') {
                let key = format!("host-{}", host);
                store_route.insert(key, route.name.clone());
            }
        }
        "header" => {
            let key = format!("header-{}", route.route.value);
            store_route.insert(key, route.name.clone());
        }
        _ => {
            return Err(NylonError::ConfigError(format!(
                "Invalid route kind: {}",
                route.route.kind
            )));
        }
    }
    Ok(())
}

fn process_route_middleware(
    route: &RouteConfig,
    middleware_groups: &HashMap<String, Vec<MiddlewareItem>>,
) -> Result<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>, NylonError> {
    let mut route_middleware = vec![];
    if let Some(middleware) = &route.middleware {
        for m in middleware {
            if let Some(group) = &m.group {
                if let Some(plugins) = middleware_groups.get(group) {
                    parsed_middleware(plugins.clone(), &mut route_middleware);
                }
                continue;
            }
            parsed_middleware(vec![m.clone()], &mut route_middleware);
        }
    }
    Ok(route_middleware)
}

fn create_matchit_router(
    route: &RouteConfig,
    services: &Vec<&ServiceItem>,
    route_middleware: &[(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)],
    middleware_groups: &HashMap<String, Vec<MiddlewareItem>>,
) -> Result<matchit::Router<Route>, NylonError> {
    let mut matchit_route = matchit::Router::<Route>::new();
    
    for path in &route.paths {
        let match_path = extract_match_path(path)?;
        let methods = path.methods.clone();
        let service = create_route_service(path, services, route_middleware, middleware_groups)?;

        if let Some(methods) = methods {
            for method in methods {
                for p in &match_path {
                    matchit_route
                        .insert(format!("/{method}{p}"), service.clone())
                        .map_err(|e| {
                            NylonError::ConfigError(format!("Failed to register route: {e}"))
                        })?;
                    tracing::info!("[{}] Add: {:?}", route.name, format!("/{method}{p}"));
                }
            }
        } else {
            for p in match_path {
                matchit_route.insert(p, service.clone()).map_err(|e| {
                    NylonError::ConfigError(format!("Failed to register route: {e}"))
                })?;
                tracing::info!("[{}] Add: {:?}", route.name, p);
            }
        }
    }
    Ok(matchit_route)
}

fn extract_match_path(path: &PathConfig) -> Result<Vec<&str>, NylonError> {
    let mut match_path: Vec<&str> = vec![];
    match &path.path {
        Value::Array(arr) => {
            for p in arr {
                match_path.push(p.as_str().unwrap_or_default());
            }
        }
        Value::String(p) => {
            match_path.push(p.as_str());
        }
        _ => {
            return Err(NylonError::ConfigError(format!(
                "Invalid path type: {}",
                path.path
            )));
        }
    }
    Ok(match_path)
}

fn create_route_service(
    path: &PathConfig,
    services: &Vec<&ServiceItem>,
    route_middleware: &[(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)],
    middleware_groups: &HashMap<String, Vec<MiddlewareItem>>,
) -> Result<Route, NylonError> {
    let mut service = Route {
        service: path.service.name.clone(),
        rewrite: path.service.rewrite.clone(),
        service_type: match services.iter().find(|s| s.name == path.service.name) {
            Some(s) => s.service_type.clone(),
            None => {
                return Err(NylonError::ConfigError(format!(
                    "Service {} not found",
                    path.service.name
                )));
            }
        },
        route_middleware: Some(route_middleware.to_vec()),
        path_middleware: None,
    };

    if let Some(middleware) = &path.middleware {
        let mut middleware_items = vec![];
        for m in middleware {
            if let Some(group) = &m.group {
                if let Some(plugins) = middleware_groups.get(group) {
                    parsed_middleware(plugins.clone(), &mut middleware_items);
                }
                continue;
            }
            parsed_middleware(vec![m.clone()], &mut middleware_items);
        }
        service.path_middleware = Some(middleware_items);
    }

    Ok(service)
}

pub fn find_route(session: &Session) -> Result<(Route, HashMap<String, String>), NylonError> {
    let (path, host, method) = get_request_info(session)?;
    let routes_matchit = get_routes_matchit()?;
    let header_selector = get_header_selector()?;
    let store_route = get_store_route()?;

    // Check header match
    if let Some(header_value) = session.req_header().headers.get(&header_selector) {
        let value = header_value.to_str().unwrap_or_default();
        if let Some(route_name) = store_route.get(&format!("header-{value}")) {
            return find_matching_route(&routes_matchit, route_name, &path, &method);
        }
    }

    // Fallback to host match
    if let Some(route_name) = store_route.get(&format!("host-{host}")) {
        return find_matching_route(&routes_matchit, route_name, &path, &method);
    }

    Err(NylonError::RouteNotFound(format!(
        "No route matched for host: {host}, method: {method}, path: {path}"
    )))
}

fn get_routes_matchit() -> Result<HashMap<String, matchit::Router<Route>>, NylonError> {
    store::get::<HashMap<String, matchit::Router<Route>>>(store::KEY_ROUTES_MATCHIT)
        .ok_or_else(|| NylonError::ShouldNeverHappen("Route matcher not found in store".into()))
}

fn get_header_selector() -> Result<String, NylonError> {
    store::get::<String>(store::KEY_HEADER_SELECTOR)
        .ok_or_else(|| NylonError::ShouldNeverHappen("Header selector not configured".into()))
}

fn get_store_route() -> Result<HashMap<String, String>, NylonError> {
    store::get::<HashMap<String, String>>(store::KEY_ROUTES)
        .ok_or_else(|| NylonError::ShouldNeverHappen("Route map not found in store".into()))
}

fn get_request_info(session: &Session) -> Result<(String, String, String), NylonError> {
    if session.is_http2() {
        get_http2_request_info(session)
    } else {
        get_http1_request_info(session)
    }
}

fn get_http2_request_info(session: &Session) -> Result<(String, String, String), NylonError> {
    let s = session.as_http2().ok_or_else(|| {
        NylonError::RouteNotFound("Failed to interpret session as HTTP/2".into())
    })?;

    Ok((
        s.req_header().uri.path().to_string(),
        s.req_header().uri.host().unwrap_or_default().to_string(),
        s.req_header().method.to_string(),
    ))
}

fn get_http1_request_info(session: &Session) -> Result<(String, String, String), NylonError> {
    let path = session.req_header().uri.path().to_string();
    let host = session
        .get_header("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default()
        .split(':')
        .next()
        .unwrap_or("")
        .to_string();
    let method = session.req_header().method.to_string();

    Ok((path, host, method))
}

fn find_matching_route(
    routes_matchit: &HashMap<String, matchit::Router<Route>>,
    route_name: &str,
    path: &str,
    method: &str,
) -> Result<(Route, HashMap<String, String>), NylonError> {
    let router = routes_matchit
        .get(route_name)
        .ok_or_else(|| NylonError::RouteNotFound("Route map missing for given name".into()))?;

    let path_with_method = format!("/{method}{path}");

    let result = router
        .at(path)
        .or_else(|_| router.at(&path_with_method))
        .map_err(|_| {
            NylonError::RouteNotFound(format!(
                "No route matched for method: {method}, path: {path}"
            ))
        })?;

    let route = result.value.clone();
    let params = result
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    Ok((route, params))
}
