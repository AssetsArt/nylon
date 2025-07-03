//! Nylon Context Types
//!
//! This module contains the core context types used throughout the Nylon proxy server,
//! including request context, route definitions, and session management.

#![allow(clippy::type_complexity)]

use crate::{plugins::SessionStream, route::MiddlewareItem, services::ServiceItem, template::Expr};
use pingora::lb::Backend;
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a matched route with its associated service and middleware
#[derive(Debug, Clone)]
pub struct Route {
    /// The service this route points to
    pub service: ServiceItem,

    /// Optional URL rewrite pattern
    pub rewrite: Option<String>,

    /// Middleware applied at the route level
    pub route_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,

    /// Middleware applied at the path level
    pub path_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,

    /// AST representation of payload expressions
    pub payload_ast: Option<HashMap<String, Vec<Expr>>>,
}

/// Main context for HTTP request processing
///
/// This struct holds all the state and data associated with a single HTTP request,
/// including routing information, middleware state, and response modifications.
#[derive(Debug, Clone)]
pub struct NylonContext {
    // Backend and routing
    /// Selected backend for this request
    pub backend: Backend,

    /// Client IP address
    pub client_ip: String,

    /// Matched route for this request
    pub route: Option<Route>,

    /// Route parameters extracted from the URL
    pub params: Option<HashMap<String, String>>,

    /// Unique request identifier
    pub request_id: String,

    /// Plugin-specific storage
    pub plugin_store: Option<Vec<u8>>,

    /// Request host header
    pub host: String,

    /// Whether the request is using TLS
    pub tls: bool,

    /// Session streams for streaming responses
    pub session_stream: HashMap<String, SessionStream>,

    // Response modifications
    /// Headers to add to the response
    pub add_response_header: HashMap<String, String>,

    /// Headers to remove from the response
    pub remove_response_header: Vec<String>,

    /// Status code to set for the response
    pub set_response_status: u16,

    /// Body content to append to the response
    pub set_response_body: Vec<u8>,

    // Request modifications
    /// Whether the request body has been read
    pub read_body: bool,

    /// Request body content
    pub request_body: Vec<u8>,
}

impl Default for NylonContext {
    fn default() -> Self {
        Self {
            // Backend and routing
            backend: Backend::new("127.0.0.1:80").expect("Unable to create default backend"),
            client_ip: "127.0.0.1".to_string(),
            route: None,
            params: None,
            request_id: Uuid::now_v7().to_string(),
            plugin_store: None,
            host: "".to_string(),
            tls: false,
            session_stream: HashMap::new(),

            // Response modifications
            add_response_header: HashMap::new(),
            remove_response_header: Vec::new(),
            set_response_status: 200,
            set_response_body: Vec::new(),

            // Request modifications
            read_body: false,
            request_body: Vec::new(),
        }
    }
}

impl NylonContext {
    /// Create a new NylonContext with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the request ID
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Get the client IP address
    pub fn client_ip(&self) -> &str {
        &self.client_ip
    }

    /// Get the request host
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Check if the request is using TLS
    pub fn is_tls(&self) -> bool {
        self.tls
    }

    /// Get route parameters
    pub fn params(&self) -> Option<&HashMap<String, String>> {
        self.params.as_ref()
    }

    /// Get a specific route parameter
    pub fn param(&self, key: &str) -> Option<&String> {
        self.params.as_ref()?.get(key)
    }

    /// Add a response header
    pub fn add_response_header(&mut self, key: String, value: String) {
        self.add_response_header.insert(key, value);
    }

    /// Remove a response header
    pub fn remove_response_header(&mut self, key: String) {
        self.remove_response_header.push(key);
    }

    /// Set the response status code
    pub fn set_response_status(&mut self, status: u16) {
        self.set_response_status = status;
    }

    /// Append content to the response body
    pub fn append_response_body(&mut self, content: &[u8]) {
        self.set_response_body.extend_from_slice(content);
    }

    /// Set the complete response body
    pub fn set_response_body(&mut self, content: Vec<u8>) {
        self.set_response_body = content;
    }

    /// Read the request body
    pub fn read_request_body(&mut self) -> &[u8] {
        self.read_body = true;
        &self.request_body
    }

    /// Set the request body
    pub fn set_request_body(&mut self, content: Vec<u8>) {
        self.request_body = content;
    }
}
