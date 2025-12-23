//! Prometheus metrics for TotalImage MCP Server
//!
//! Provides instrumentation for request latency, error rates, cache hits, and more.

use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, Encoder, HistogramVec,
    IntCounterVec, IntGauge, TextEncoder,
};
use std::sync::Arc;

lazy_static::lazy_static! {
    /// Total number of tool calls by tool name and status
    pub static ref TOOL_CALLS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "totalimage_mcp_tool_calls_total",
        "Total number of tool calls by tool name and status (success/error)",
        &["tool", "status"]
    ).unwrap();

    /// Tool execution duration in seconds
    pub static ref TOOL_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        "totalimage_mcp_tool_duration_seconds",
        "Tool execution duration in seconds",
        &["tool"],
        vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0]
    ).unwrap();

    /// Cache hits vs misses
    pub static ref CACHE_OPERATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "totalimage_mcp_cache_operations_total",
        "Total cache operations by operation type (hit/miss)",
        &["operation"]
    ).unwrap();

    /// Current cache size in bytes
    pub static ref CACHE_SIZE_BYTES: IntGauge = register_int_gauge!(
        "totalimage_mcp_cache_size_bytes",
        "Current size of the tool result cache in bytes"
    ).unwrap();

    /// Active requests currently being processed
    pub static ref ACTIVE_REQUESTS: IntGauge = register_int_gauge!(
        "totalimage_mcp_active_requests",
        "Number of requests currently being processed"
    ).unwrap();

    /// HTTP request count by method and status
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "totalimage_mcp_http_requests_total",
        "Total HTTP requests by method and status code",
        &["method", "status"]
    ).unwrap();

    /// WebSocket connections
    pub static ref WEBSOCKET_CONNECTIONS: IntGauge = register_int_gauge!(
        "totalimage_mcp_websocket_connections",
        "Number of active WebSocket connections"
    ).unwrap();

    /// Vault operations (open, read, seek)
    pub static ref VAULT_OPERATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "totalimage_mcp_vault_operations_total",
        "Total vault operations by vault type and operation",
        &["vault_type", "operation"]
    ).unwrap();

    /// Filesystem operations
    pub static ref FILESYSTEM_OPERATIONS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "totalimage_mcp_filesystem_operations_total",
        "Total filesystem operations by filesystem type and operation",
        &["fs_type", "operation"]
    ).unwrap();
}

/// Metrics handler state
#[derive(Clone)]
pub struct MetricsState {
    encoder: Arc<TextEncoder>,
}

impl MetricsState {
    pub fn new() -> Self {
        Self {
            encoder: Arc::new(TextEncoder::new()),
        }
    }

    /// Encode all metrics in Prometheus text format
    pub fn encode(&self) -> Result<String, prometheus::Error> {
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        self.encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }
}

impl Default for MetricsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Record a tool call completion
pub fn record_tool_call(tool_name: &str, success: bool) {
    let status = if success { "success" } else { "error" };
    TOOL_CALLS_TOTAL
        .with_label_values(&[tool_name, status])
        .inc();
}

/// Record tool execution duration
pub fn record_tool_duration(tool_name: &str, duration_secs: f64) {
    TOOL_DURATION_SECONDS
        .with_label_values(&[tool_name])
        .observe(duration_secs);
}

/// Record cache operation
pub fn record_cache_operation(hit: bool) {
    let operation = if hit { "hit" } else { "miss" };
    CACHE_OPERATIONS_TOTAL.with_label_values(&[operation]).inc();
}

/// Update cache size metric
pub fn update_cache_size(size_bytes: i64) {
    CACHE_SIZE_BYTES.set(size_bytes);
}

/// Increment active requests
pub fn increment_active_requests() {
    ACTIVE_REQUESTS.inc();
}

/// Decrement active requests
pub fn decrement_active_requests() {
    ACTIVE_REQUESTS.dec();
}

/// Record HTTP request
pub fn record_http_request(method: &str, status: u16) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, &status.to_string()])
        .inc();
}

/// Set WebSocket connection count
pub fn set_websocket_connections(count: i64) {
    WEBSOCKET_CONNECTIONS.set(count);
}

/// Record vault operation
pub fn record_vault_operation(vault_type: &str, operation: &str) {
    VAULT_OPERATIONS_TOTAL
        .with_label_values(&[vault_type, operation])
        .inc();
}

/// Record filesystem operation
pub fn record_filesystem_operation(fs_type: &str, operation: &str) {
    FILESYSTEM_OPERATIONS_TOTAL
        .with_label_values(&[fs_type, operation])
        .inc();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_encoding() {
        // Record some metrics first to ensure they appear in the output
        record_tool_call("test_tool", true);
        record_cache_operation(true);

        let state = MetricsState::new();
        let encoded = state.encode();
        assert!(encoded.is_ok());
        let text = encoded.unwrap();
        // Check for any of our metric names
        assert!(
            text.contains("totalimage_mcp")
                || text.contains("totalimage_mcp_tool_calls_total")
                || text.contains("totalimage_mcp_cache_operations_total"),
            "Metrics output should contain totalimage_mcp metrics, got: {}",
            text
        );
    }

    #[test]
    fn test_tool_call_recording() {
        record_tool_call("analyze_disk_image", true);
        record_tool_call("analyze_disk_image", false);

        let metric_families = prometheus::gather();
        let tool_calls = metric_families
            .iter()
            .find(|mf| mf.name == "totalimage_mcp_tool_calls_total");
        assert!(tool_calls.is_some());
    }

    #[test]
    fn test_cache_operations() {
        record_cache_operation(true);
        record_cache_operation(false);

        let metric_families = prometheus::gather();
        let cache_ops = metric_families
            .iter()
            .find(|mf| mf.name == "totalimage_mcp_cache_operations_total");
        assert!(cache_ops.is_some());
    }

    #[test]
    fn test_active_requests() {
        let initial = ACTIVE_REQUESTS.get();
        increment_active_requests();
        assert_eq!(ACTIVE_REQUESTS.get(), initial + 1);
        decrement_active_requests();
        assert_eq!(ACTIVE_REQUESTS.get(), initial);
    }
}
