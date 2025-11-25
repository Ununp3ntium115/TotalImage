//! Transport layer for tool communication
//!
//! Supports HTTP transport for calling remote tools

use crate::registry::{AuthConfig, RegisteredTool, ToolExecutor};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Request to call a tool method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// Tool name
    pub tool: String,
    /// Method name
    pub method: String,
    /// Method arguments
    pub arguments: Value,
    /// Request ID for tracking
    #[serde(default)]
    pub request_id: Option<String>,
}

/// Response from a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    /// Whether the call succeeded
    pub success: bool,
    /// Result data (if success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error message (if failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl ToolCallResponse {
    /// Create a successful response
    pub fn success(result: Value, duration_ms: u64) -> Self {
        Self {
            success: true,
            result: Some(result),
            error: None,
            duration_ms: Some(duration_ms),
        }
    }

    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            result: None,
            error: Some(message.into()),
            duration_ms: None,
        }
    }
}

/// HTTP transport for calling tools
pub struct HttpTransport {
    client: reqwest::Client,
    timeout: Duration,
}

impl HttpTransport {
    /// Create a new HTTP transport
    pub fn new(timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            timeout: Duration::from_secs(timeout_secs),
        }
    }

    /// Call a tool method via HTTP
    pub async fn call(
        &self,
        tool: &RegisteredTool,
        request: &ToolCallRequest,
    ) -> Result<ToolCallResponse> {
        let start = std::time::Instant::now();

        // Get URL from executor
        let (base_url, auth) = match &tool.info.executor {
            ToolExecutor::Http { url, auth } => (url.clone(), auth.clone()),
            _ => {
                return Err(Error::ExecutionFailed(
                    "Tool is not configured for HTTP transport".to_string(),
                ))
            }
        };

        // Build the full URL
        let url = format!("{}/tools/call", base_url);

        // Build request
        let mut req = self
            .client
            .post(&url)
            .json(request)
            .timeout(self.timeout);

        // Add authentication if configured
        if let Some(auth_config) = auth {
            req = match auth_config {
                AuthConfig::Bearer { token } => {
                    req.header("Authorization", format!("Bearer {}", token))
                }
                AuthConfig::ApiKey { header, key } => req.header(header, key),
            };
        }

        // Send request
        let response = req.send().await.map_err(|e| {
            Error::Http(format!("Request failed: {}", e))
        })?;

        // Check status
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Ok(ToolCallResponse::error(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        // Parse response
        let result: Value = response.json().await.map_err(|e| {
            Error::Http(format!("Failed to parse response: {}", e))
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ToolCallResponse::success(result, duration_ms))
    }

    /// Health check a tool
    pub async fn health_check(&self, tool: &RegisteredTool) -> Result<bool> {
        let base_url = match &tool.info.executor {
            ToolExecutor::Http { url, .. } => url.clone(),
            _ => return Ok(false),
        };

        let url = format!("{}/health", base_url);

        match self.client.get(&url).timeout(Duration::from_secs(5)).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new(30) // 30 second default timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_response_success() {
        let response = ToolCallResponse::success(serde_json::json!({"data": "test"}), 100);
        assert!(response.success);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_tool_call_response_error() {
        let response = ToolCallResponse::error("Something went wrong");
        assert!(!response.success);
        assert!(response.result.is_none());
        assert_eq!(response.error, Some("Something went wrong".to_string()));
    }
}
