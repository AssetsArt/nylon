use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;

pub type MessageHeaders = BTreeMap<String, String>;
pub type RequestId = u128;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseAction {
    Next,
    End,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRequest {
    pub version: u16,
    pub request_id: RequestId,
    pub session_id: u32,
    pub phase: u8,
    pub method: u32,
    pub data: Vec<u8>,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<MessageHeaders>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    pub version: u16,
    pub request_id: RequestId,
    pub session_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<u32>,
    pub action: ResponseAction,
    pub data: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<MessageHeaders>,
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("failed to encode messagepack payload: {0}")]
    Encode(#[from] rmp_serde::encode::Error),
    #[error("failed to decode messagepack payload: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
}

pub fn new_request_id() -> RequestId {
    Uuid::now_v7().as_u128()
}

pub fn decode_request(bytes: &[u8]) -> Result<PluginRequest, ProtocolError> {
    Ok(rmp_serde::from_slice(bytes)?)
}

pub fn encode_response(response: &PluginResponse) -> Result<Vec<u8>, ProtocolError> {
    Ok(rmp_serde::to_vec(response)?)
}

pub fn decode_response(bytes: &[u8]) -> Result<PluginResponse, ProtocolError> {
    Ok(rmp_serde::from_slice(bytes)?)
}

pub fn encode_request(request: &PluginRequest) -> Result<Vec<u8>, ProtocolError> {
    Ok(rmp_serde::to_vec(request)?)
}
