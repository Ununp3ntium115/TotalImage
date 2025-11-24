//! Model Context Protocol (MCP) message types and protocol implementation
//!
//! This module defines the MCP protocol messages, request/response types,
//! and error handling for communication with Claude Desktop or Fire Marshal.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP protocol version supported by this server
pub const MCP_VERSION: &str = "2024-11-05";

/// MCP request message (from client to server)
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum MCPRequest {
    /// Initialize the MCP connection
    #[serde(rename = "initialize")]
    Initialize {
        jsonrpc: String,
        id: RequestId,
        params: InitializeParams,
    },

    /// List available tools
    #[serde(rename = "tools/list")]
    ListTools {
        jsonrpc: String,
        id: RequestId,
    },

    /// Call a specific tool
    #[serde(rename = "tools/call")]
    CallTool {
        jsonrpc: String,
        id: RequestId,
        params: CallToolParams,
    },
}

/// Request ID (can be string or number)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}

/// MCP response message (from server to client)
#[derive(Debug, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MCPError>,
}

impl MCPResponse {
    pub fn success(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: RequestId, error: MCPError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// MCP error response
#[derive(Debug, Serialize, Deserialize)]
pub struct MCPError {
    pub code: MCPErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl MCPError {
    pub fn tool_not_found(tool_name: &str) -> Self {
        Self {
            code: MCPErrorCode::MethodNotFound,
            message: format!("Tool '{}' not found", tool_name),
            data: None,
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: MCPErrorCode::InvalidParams,
            message: message.into(),
            data: None,
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: MCPErrorCode::InternalError,
            message: message.into(),
            data: None,
        }
    }
}

/// MCP error codes (JSON-RPC 2.0 standard)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(i32)]
pub enum MCPErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
}

/// Initialize request parameters
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: Capabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

/// Client capabilities
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Capabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Client information
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Call tool request parameters
#[derive(Debug, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// Tool execution result
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "isError")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    pub fn success(content: Vec<Content>) -> Self {
        Self {
            content,
            is_error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![Content::text(message)],
            is_error: Some(true),
        }
    }

    pub fn from_value(value: Value) -> Self {
        Self {
            content: vec![Content::json(value)],
            is_error: None,
        }
    }
}

/// Content block in tool result
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Content {
    Text {
        text: String,
    },
    #[serde(rename = "resource")]
    Resource {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        blob: Option<String>,
    },
}

impl Content {
    pub fn text(text: impl Into<String>) -> Self {
        Content::Text { text: text.into() }
    }

    pub fn json(value: Value) -> Self {
        Content::Text {
            text: serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()),
        }
    }

    pub fn resource_text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Content::Resource {
            uri: uri.into(),
            text: Some(text.into()),
            blob: None,
        }
    }
}

/// Tool definition for tools/list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Initialize response
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeResponse {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

/// Server capabilities
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: Option<ServerToolCapabilities>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerToolCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Server information
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}
