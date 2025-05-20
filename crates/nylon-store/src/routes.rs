use crate as store;
use nylon_error::NylonError;
use nylon_types::route::{PathType, RouteConfig};
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
                    let key = format!("/{}/{}", method, match_path);
                    matchit_route
                        .insert(
                            key,
                            Route {
                                service: path.service.name.clone(),
                                rewrite: path.service.rewrite.clone(),
                            },
                        )
                        .map_err(|e| {
                            NylonError::ConfigError(format!("Failed to insert route: {}", e))
                        })?;
                }
            } else {
                matchit_route
                    .insert(
                        match_path,
                        Route {
                            service: path.service.name.clone(),
                            rewrite: path.service.rewrite.clone(),
                        },
                    )
                    .map_err(|e| {
                        NylonError::ConfigError(format!("Failed to insert route: {}", e))
                    })?;
            }
        }
        let mut m_route = HashMap::new();
        m_route.insert(route.name.clone(), matchit_route);
        store::insert(store::KEY_ROUTES_MATCHIT, m_route);
    }
    store::insert(store::KEY_ROUTES, store_route);
    Ok(())
}
