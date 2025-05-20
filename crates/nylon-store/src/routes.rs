use crate as store;
use nylon_error::NylonError;
use nylon_types::route::{PathType, RouteConfig};
use pingora::{http::Version, proxy::Session};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Route {
    pub service: String,
    pub rewrite: Option<String>,
}

pub fn store(routes: Vec<&RouteConfig>) -> Result<(), NylonError> {
    let mut store_route: HashMap<String, String> = HashMap::new();
    for route in routes {
        if route.route.kind == "host" {
            let hosts: Vec<&str> = route.route.value.split('|').collect();
            for host in hosts {
                let key = format!("{}-{}", route.route.kind, host);
                store_route.insert(key, route.name.clone());
            }
        } else if route.route.kind == "header" {
            let key = format!("{}-{}", route.route.kind, route.route.value);
            store_route.insert(key, route.name.clone());
        }

        let mut matchit_route = matchit::Router::<Route>::new();
        for path in route.paths.clone() {
            let mut match_path = path.path.clone();
            if path.path_type == PathType::Prefix {
                match_path = format!("{}/{{*p}}", path.path.clone());
                if path.path.clone() == *"/" {
                    match_path = "/{*p}".to_string();
                }
            }
            if let Some(methods) = path.methods {
                for method in methods {
                    if path.path.clone() == *"/" {
                        let match_path_with_method = format!("/{}{}", method, path.path.clone());
                        matchit_route
                            .insert(
                                match_path_with_method.clone(),
                                Route {
                                    service: path.service.name.clone(),
                                    rewrite: path.service.rewrite.clone(),
                                },
                            )
                            .map_err(|e| {
                                NylonError::ConfigError(format!("Failed to insert route: {}", e))
                            })?;
                        tracing::info!("[{}] Add: {:?}", route.name, match_path_with_method);
                    }
                    let match_path_with_method = format!("/{}{}", method, match_path);
                    matchit_route
                        .insert(
                            match_path_with_method.clone(),
                            Route {
                                service: path.service.name.clone(),
                                rewrite: path.service.rewrite.clone(),
                            },
                        )
                        .map_err(|e| {
                            NylonError::ConfigError(format!("Failed to insert route: {}", e))
                        })?;

                    tracing::info!("[{}] Add: {:?}", route.name, match_path_with_method);
                }
            } else {
                if path.path.clone() == *"/" {
                    matchit_route
                        .insert(
                            path.path.clone(),
                            Route {
                                service: path.service.name.clone(),
                                rewrite: path.service.rewrite.clone(),
                            },
                        )
                        .map_err(|e| {
                            NylonError::ConfigError(format!("Failed to insert route: {}", e))
                        })?;
                    tracing::info!("[{}] Add: {:?}", route.name, path.path.clone());
                }
                matchit_route
                    .insert(
                        match_path.clone(),
                        Route {
                            service: path.service.name.clone(),
                            rewrite: path.service.rewrite.clone(),
                        },
                    )
                    .map_err(|e| {
                        NylonError::ConfigError(format!("Failed to insert route: {}", e))
                    })?;
                tracing::info!("[{}] Add: {:?}", route.name, match_path);
            }
        }
        let mut m_route = HashMap::new();
        m_route.insert(route.name.clone(), matchit_route);
        store::insert(store::KEY_ROUTES_MATCHIT, m_route);
    }
    store::insert(store::KEY_ROUTES, store_route);
    Ok(())
}

pub fn find_route(session: &Session) -> Result<(Route, HashMap<String, String>), NylonError> {
    let mut path = session.req_header().uri.path().to_string();
    let mut host = match session.get_header("host") {
        Some(host) => host.to_str().unwrap_or_default().to_string(),
        None => "".to_string(),
    };
    let mut method = session.req_header().method.to_string();
    if session.req_header().version == Version::HTTP_2 {
        let sessionv2 = match session.as_http2() {
            Some(session) => session,
            None => {
                return Err(NylonError::RouteNotFound(
                    "Session is not HTTP/2".to_string(),
                ));
            }
        };
        path = sessionv2.req_header().uri.path().to_string();
        host = match sessionv2.req_header().uri.host() {
            Some(h) => h.to_string(),
            None => "".to_string(),
        };
        method = sessionv2.req_header().method.to_string();
    } else {
        host = match host.split(':').next() {
            Some(h) => h.to_string(),
            None => host,
        };
    }
    let Some(routes_matchit) =
        store::get::<HashMap<String, matchit::Router<Route>>>(store::KEY_ROUTES_MATCHIT)
    else {
        return Err(NylonError::RouteNotFound("Routes are not set".to_string()));
    };
    let Some(header_selector) = store::get::<String>(store::KEY_HEADER_SELECTOR) else {
        return Err(NylonError::RouteNotFound(
            "Header selector is not set".to_string(),
        ));
    };
    let Some(store_route) = store::get::<HashMap<String, String>>(store::KEY_ROUTES) else {
        return Err(NylonError::RouteNotFound("Routes are not set".to_string()));
    };
    // println!("method: {:?}", method);
    let header_selector = session.req_header().headers.get(header_selector.as_str());
    if let Some(header_selector) = header_selector {
        let header_selector = header_selector.to_str().unwrap_or_default().to_string();
        let route = store_route.get::<String>(&format!("header-{}", header_selector));
        if let Some(route_name) = route {
            let Some(route) = routes_matchit.get(route_name) else {
                return Err(NylonError::RouteNotFound("Route is not set".to_string()));
            };
            let path_with_method = format!("/{}{}", method, path);
            let matchit_route = match route.at(path.as_str()) {
                Ok(route) => route,
                Err(_) => match route.at(path_with_method.as_str()) {
                    Ok(route) => route,
                    Err(_) => {
                        return Err(NylonError::RouteNotFound("Route is not set".to_string()));
                    }
                },
            };
            let route = matchit_route.value.clone();
            let params = matchit_route.params.clone();
            let mut params_map = HashMap::new();
            for (key, value) in params.iter() {
                params_map.insert(key.to_string(), value.to_string());
            }
            return Ok((route, params_map));
        }
    }

    let route = store_route.get::<String>(&format!("host-{}", host));
    if let Some(route_name) = route {
        let Some(route) = routes_matchit.get(route_name) else {
            return Err(NylonError::RouteNotFound("Route is not set".to_string()));
        };
        let path_with_method = format!("/{}{}", method, path);
        // println!("path: {:?}", path);
        // println!("path_with_method: {:?}", path_with_method);
        let matchit_route = match route.at(path.as_str()) {
            Ok(route) => route,
            Err(_) => match route.at(path_with_method.as_str()) {
                Ok(route) => route,
                Err(_) => {
                    return Err(NylonError::RouteNotFound("Route is not set".to_string()));
                }
            },
        };
        let route = matchit_route.value.clone();
        let params = matchit_route.params.clone();
        let mut params_map = HashMap::new();
        for (key, value) in params.iter() {
            params_map.insert(key.to_string(), value.to_string());
        }
        return Ok((route, params_map));
    }
    Err(NylonError::RouteNotFound("Route is not set".to_string()))
}
