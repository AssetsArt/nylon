use crate as store;
use nylon_error::NylonError;
use nylon_types::route::{PathType, RouteConfig};
use pingora::proxy::Session;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Route {
    pub service: String,
    pub rewrite: Option<String>,
}

pub fn store(routes: Vec<&RouteConfig>) -> Result<(), NylonError> {
    let mut store_route = HashMap::new();

    for route in routes {
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

        let mut matchit_route = matchit::Router::<Route>::new();

        for path in &route.paths {
            let mut match_path = path.path.clone();
            if path.path_type == PathType::Prefix {
                match_path = if path.path == "/" {
                    "/{*p}".to_string()
                } else {
                    format!("{}/{{*p}}", path.path)
                };
            }

            let methods = path.methods.clone();
            let service = Route {
                service: path.service.name.clone(),
                rewrite: path.service.rewrite.clone(),
            };

            if let Some(methods) = methods {
                for method in methods {
                    let mut match_path_with_method = vec![];
                    if path.path == "/" && path.path_type == PathType::Prefix {
                        match_path_with_method.push(format!("/{method}/"));
                    }
                    match_path_with_method.push(format!("/{method}{match_path}"));

                    for path in match_path_with_method {
                        matchit_route.insert(&path, service.clone()).map_err(|e| {
                            NylonError::ConfigError(format!("Failed to register route: {e}"))
                        })?;
                        tracing::info!("[{}] Add: {:?}", route.name, path);
                    }
                }
            } else {
                let mut add_path = vec![];
                if path.path == "/" && path.path_type == PathType::Prefix {
                    add_path.push("/".to_string());
                }
                add_path.push(match_path);

                for p in add_path {
                    matchit_route.insert(&p, service.clone()).map_err(|e| {
                        NylonError::ConfigError(format!("Failed to register route: {e}"))
                    })?;
                    tracing::info!("[{}] Add: {:?}", route.name, p);
                }
            }
        }

        store::insert(
            store::KEY_ROUTES_MATCHIT,
            HashMap::from([(route.name.clone(), matchit_route)]),
        );
    }

    store::insert(store::KEY_ROUTES, store_route);
    Ok(())
}

pub fn find_route(session: &Session) -> Result<(Route, HashMap<String, String>), NylonError> {
    let (path, host, method) = get_request_info(session)?;

    let routes_matchit =
        store::get::<HashMap<String, matchit::Router<Route>>>(store::KEY_ROUTES_MATCHIT)
            .ok_or_else(|| {
                NylonError::ShouldNeverHappen("Route matcher not found in store".into())
            })?;

    let header_selector = store::get::<String>(store::KEY_HEADER_SELECTOR)
        .ok_or_else(|| NylonError::ShouldNeverHappen("Header selector not configured".into()))?;

    let store_route = store::get::<HashMap<String, String>>(store::KEY_ROUTES)
        .ok_or_else(|| NylonError::ShouldNeverHappen("Route map not found in store".into()))?;

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

fn get_request_info(session: &Session) -> Result<(String, String, String), NylonError> {
    if session.is_http2() {
        let s = session.as_http2().ok_or_else(|| {
            NylonError::RouteNotFound("Failed to interpret session as HTTP/2".into())
        })?;

        Ok((
            s.req_header().uri.path().to_string(),
            s.req_header().uri.host().unwrap_or_default().to_string(),
            s.req_header().method.to_string(),
        ))
    } else {
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
