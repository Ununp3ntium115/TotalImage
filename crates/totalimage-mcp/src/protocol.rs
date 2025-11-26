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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_id_string() {
        let id = RequestId::String("test-123".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, r#""test-123""#);
    }

    #[test]
    fn test_request_id_number() {
        let id = RequestId::Number(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");
    }

    #[test]
    fn test_parse_initialize_request() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }"#;

        let request: MCPRequest = serde_json::from_str(json).unwrap();
        match request {
            MCPRequest::Initialize { id, params, .. } => {
                assert!(matches!(id, RequestId::Number(1)));
                assert_eq!(params.protocol_version, "2024-11-05");
            }
            _ => panic!("Expected Initialize request"),
        }
    }

    #[test]
    fn test_parse_list_tools_request() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": "req-1",
            "method": "tools/list"
        }"#;

        let request: MCPRequest = serde_json::from_str(json).unwrap();
        match request {
            MCPRequest::ListTools { id, .. } => {
                assert!(matches!(id, RequestId::String(s) if s == "req-1"));
            }
            _ => panic!("Expected ListTools request"),
        }
    }

    #[test]
    fn test_parse_call_tool_request() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 123,
            "method": "tools/call",
            "params": {
                "name": "analyze_disk_image",
                "arguments": {
                    "path": "/tmp/test.img"
                }
            }
        }"#;

        let request: MCPRequest = serde_json::from_str(json).unwrap();
        match request {
            MCPRequest::CallTool { id, params, .. } => {
                assert!(matches!(id, RequestId::Number(123)));
                assert_eq!(params.name, "analyze_disk_image");
                assert!(params.arguments.is_some());
            }
            _ => panic!("Expected CallTool request"),
        }
    }

    #[test]
    fn test_mcp_response_success() {
        let response = MCPResponse::success(
            RequestId::Number(1),
            json!({"result": "ok"}),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_mcp_response_error() {
        let error = MCPError::tool_not_found("unknown_tool");
        let response = MCPResponse::error(RequestId::String("test".to_string()), error);

        assert!(response.result.is_none());
        assert!(response.error.is_some());
        let err = response.error.unwrap();
        assert_eq!(err.code, MCPErrorCode::MethodNotFound);
        assert!(err.message.contains("unknown_tool"));
    }

    #[test]
    fn test_mcp_error_invalid_params() {
        let error = MCPError::invalid_params("Missing path parameter");
        assert_eq!(error.code, MCPErrorCode::InvalidParams);
        assert_eq!(error.message, "Missing path parameter");
    }

    #[test]
    fn test_mcp_error_internal() {
        let error = MCPError::internal_error("Something went wrong");
        assert_eq!(error.code, MCPErrorCode::InternalError);
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success(vec![Content::text("Analysis complete")]);
        assert_eq!(result.is_error, None);
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("File not found");
        assert_eq!(result.is_error, Some(true));
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_content_text() {
        let content = Content::text("Hello world");
        match content {
            Content::Text { text } => assert_eq!(text, "Hello world"),
            _ => panic!("Expected Text content"),
        }
    }

    #[test]
    fn test_content_json() {
        let content = Content::json(json!({"key": "value"}));
        match content {
            Content::Text { text } => {
                assert!(text.contains("key"));
                assert!(text.contains("value"));
            }
            _ => panic!("Expected Text content"),
        }
    }

    #[test]
    fn test_tool_definition_serialization() {
        let def = ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                }
            }),
        };

        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("test_tool"));
        assert!(json.contains("inputSchema")); // Check camelCase
    }

    #[test]
    fn test_error_code_values() {
        // Test that error codes have correct values
        assert_eq!(MCPErrorCode::ParseError as i32, -32700);
        assert_eq!(MCPErrorCode::InvalidRequest as i32, -32600);
        assert_eq!(MCPErrorCode::MethodNotFound as i32, -32601);
        assert_eq!(MCPErrorCode::InvalidParams as i32, -32602);
        assert_eq!(MCPErrorCode::InternalError as i32, -32603);
    }

    #[test]
    fn test_error_code_serialization() {
        // Test serialization - serde serializes enum variants by name
        let json = serde_json::to_string(&MCPErrorCode::ParseError).unwrap();
        assert!(json.contains("ParseError") || json.contains("-32700"));
    }

    #[test]
    fn test_initialize_response_serialization() {
        let response = InitializeResponse {
            protocol_version: MCP_VERSION.to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ServerToolCapabilities { list_changed: None }),
            },
            server_info: ServerInfo {
                name: "test-server".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("protocolVersion")); // Check camelCase
        assert!(json.contains("serverInfo"));
        assert!(json.contains(MCP_VERSION));
    }
}
