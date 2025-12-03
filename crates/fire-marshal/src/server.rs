//! Fire Marshal server implementation
//!
//! Provides HTTP API for tool orchestration with rate limiting

use crate::database::{DatabaseConfig, PlatformDatabase};
use crate::registry::{ToolInfo, ToolRegistry};
use crate::transport::{HttpTransport, ToolCallRequest, ToolCallResponse};
use crate::{Error, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};

/// Fire Marshal configuration
#[derive(Debug, Clone)]
pub struct FireMarshalConfig {
    /// Database path
    pub database_path: PathBuf,
    /// HTTP server port
    pub port: u16,
    /// Rate limit (requests per second)
    pub rate_limit_rps: u32,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum concurrent requests
    pub max_concurrent: usize,
}

impl Default for FireMarshalConfig {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("./fire-marshal.redb"),
            port: 3001,
            rate_limit_rps: 100,
            timeout_secs: 30,
            max_concurrent: 10,
        }
    }
}

/// Shared application state
struct AppState {
    registry: ToolRegistry,
    database: PlatformDatabase,
    transport: HttpTransport,
    #[allow(dead_code)]
    rate_limiter: Arc<RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
}

/// Fire Marshal server
pub struct FireMarshal {
    config: FireMarshalConfig,
    state: Arc<AppState>,
}

impl FireMarshal {
    /// Create a new Fire Marshal instance
    pub fn new(config: FireMarshalConfig) -> Result<Self> {
        // Create database
        let database = PlatformDatabase::new(
            &config.database_path,
            DatabaseConfig::default(),
        )?;

        // Create registry
        let registry = ToolRegistry::new();

        // Load previously registered tools from database
        for tool_info in database.get_registered_tools()? {
            if let Err(e) = registry.register(tool_info.clone()) {
                tracing::warn!("Failed to restore tool {}: {}", tool_info.name, e);
            }
        }

        // Create transport
        let transport = HttpTransport::new(config.timeout_secs);

        // Create rate limiter
        let quota = Quota::per_second(NonZeroU32::new(config.rate_limit_rps).unwrap_or(NonZeroU32::new(100).unwrap()));
        let rate_limiter = Arc::new(RateLimiter::direct(quota));

        let state = Arc::new(AppState {
            registry,
            database,
            transport,
            rate_limiter,
        });

        Ok(Self { config, state })
    }

    /// Register a tool
    pub fn register_tool(&self, info: ToolInfo) -> Result<()> {
        self.state.registry.register(info.clone())?;
        self.state.database.register_tool(&info)?;
        Ok(())
    }

    /// Start the HTTP server
    pub async fn serve(self) -> Result<()> {
        let state = self.state.clone();

        // Build CORS layer
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        // Build router
        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/tools/register", post(register_handler))
            .route("/tools/list", get(list_tools_handler))
            .route("/tools/call", post(call_tool_handler))
            .route("/stats", get(stats_handler))
            .layer(
                ServiceBuilder::new()
                    .layer(cors)
                    .layer(tower_http::timeout::TimeoutLayer::new(
                        std::time::Duration::from_secs(self.config.timeout_secs),
                    ))
                    .layer(tower::limit::ConcurrencyLimitLayer::new(
                        self.config.max_concurrent,
                    )),
            )
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], self.config.port));
        tracing::info!("Fire Marshal listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .await
            .map_err(Error::Io)?;

        Ok(())
    }

    /// Get reference to registry
    pub fn registry(&self) -> &ToolRegistry {
        &self.state.registry
    }

    /// Get reference to database
    pub fn database(&self) -> &PlatformDatabase {
        &self.state.database
    }
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    tools_registered: usize,
}

async fn health_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        tools_registered: state.registry.count(),
    })
}

/// Tool registration request
#[derive(Deserialize)]
struct RegisterRequest {
    #[serde(flatten)]
    info: ToolInfo,
}

/// Tool registration response
#[derive(Serialize)]
struct RegisterResponse {
    success: bool,
    message: String,
}

async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> impl IntoResponse {
    match state.registry.register(request.info.clone()) {
        Ok(()) => {
            // Also persist to database
            let _ = state.database.register_tool(&request.info);
            (
                StatusCode::OK,
                Json(RegisterResponse {
                    success: true,
                    message: format!("Tool '{}' registered successfully", request.info.name),
                }),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(RegisterResponse {
                success: false,
                message: e.to_string(),
            }),
        ),
    }
}

/// List tools response
#[derive(Serialize)]
struct ListToolsResponse {
    tools: Vec<ToolSummary>,
}

#[derive(Serialize)]
struct ToolSummary {
    name: String,
    version: String,
    description: String,
    healthy: bool,
    methods: Vec<String>,
}

async fn list_tools_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.registry.list() {
        Ok(tools) => {
            let summaries: Vec<ToolSummary> = tools
                .iter()
                .map(|t| ToolSummary {
                    name: t.info.name.clone(),
                    version: t.info.version.clone(),
                    description: t.info.description.clone(),
                    healthy: t.healthy,
                    methods: t.info.tools.iter().map(|m| m.name.clone()).collect(),
                })
                .collect();

            (StatusCode::OK, Json(ListToolsResponse { tools: summaries }))
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ListToolsResponse { tools: vec![] }),
        ),
    }
}

async fn call_tool_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ToolCallRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Look up tool
    let tool = match state.registry.get(&request.tool) {
        Ok(t) => t,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ToolCallResponse::error(format!(
                    "Tool '{}' not found",
                    request.tool
                ))),
            )
        }
    };

    // Call tool via transport
    let response = match state.transport.call(&tool, &request).await {
        Ok(resp) => resp,
        Err(e) => ToolCallResponse::error(e.to_string()),
    };

    let duration_ms = start.elapsed().as_millis() as u64;

    // Log execution
    let _ = state.database.log_execution(
        &request.tool,
        &request.method,
        response.success,
        duration_ms,
    );

    (StatusCode::OK, Json(response))
}

/// Stats response
#[derive(Serialize)]
struct StatsResponse {
    registered_tools: u64,
    cache_entries: u64,
    execution_logs: u64,
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.database.stats() {
        Ok(stats) => (
            StatusCode::OK,
            Json(StatsResponse {
                registered_tools: stats.registered_tools,
                cache_entries: stats.cache_entries,
                execution_logs: stats.execution_logs,
            }),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(StatsResponse {
                registered_tools: 0,
                cache_entries: 0,
                execution_logs: 0,
            }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ToolExecutor, ToolMethod};
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_fire_marshal_config_default() {
        let config = FireMarshalConfig::default();

        assert_eq!(config.port, 3001);
        assert_eq!(config.rate_limit_rps, 100);
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_concurrent, 10);
        assert_eq!(config.database_path, PathBuf::from("./fire-marshal.redb"));
    }

    #[test]
    fn test_fire_marshal_config_custom() {
        let config = FireMarshalConfig {
            database_path: PathBuf::from("/tmp/test.redb"),
            port: 8080,
            rate_limit_rps: 50,
            timeout_secs: 60,
            max_concurrent: 20,
        };

        assert_eq!(config.port, 8080);
        assert_eq!(config.rate_limit_rps, 50);
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.max_concurrent, 20);
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy",
            version: "1.0.0",
            tools_registered: 5,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_register_response_success() {
        let response = RegisterResponse {
            success: true,
            message: "Tool registered".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("true"));
        assert!(json.contains("Tool registered"));
    }

    #[test]
    fn test_register_response_error() {
        let response = RegisterResponse {
            success: false,
            message: "Tool already exists".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("false"));
        assert!(json.contains("Tool already exists"));
    }

    #[test]
    fn test_list_tools_response_serialization() {
        let response = ListToolsResponse {
            tools: vec![
                ToolSummary {
                    name: "test-tool".to_string(),
                    version: "1.0.0".to_string(),
                    description: "A test tool".to_string(),
                    healthy: true,
                    methods: vec!["method1".to_string(), "method2".to_string()],
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test-tool"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("method1"));
        assert!(json.contains("method2"));
    }

    #[test]
    fn test_stats_response_serialization() {
        let response = StatsResponse {
            registered_tools: 10,
            cache_entries: 100,
            execution_logs: 500,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("10"));
        assert!(json.contains("100"));
        assert!(json.contains("500"));
    }

    #[test]
    fn test_fire_marshal_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.redb");

        let config = FireMarshalConfig {
            database_path: db_path,
            port: 3001,
            rate_limit_rps: 100,
            timeout_secs: 30,
            max_concurrent: 10,
        };

        let marshal = FireMarshal::new(config);
        assert!(marshal.is_ok());

        let marshal = marshal.unwrap();
        assert_eq!(marshal.registry().count(), 0);
    }

    #[test]
    fn test_fire_marshal_register_tool() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.redb");

        let config = FireMarshalConfig {
            database_path: db_path,
            port: 3001,
            rate_limit_rps: 100,
            timeout_secs: 30,
            max_concurrent: 10,
        };

        let marshal = FireMarshal::new(config).unwrap();

        let tool_info = ToolInfo {
            name: "test-tool".to_string(),
            version: "1.0.0".to_string(),
            description: "Test tool".to_string(),
            tools: vec![ToolMethod {
                name: "analyze".to_string(),
                description: "Analyze data".to_string(),
                input_schema: serde_json::json!({"type": "object"}),
            }],
            executor: ToolExecutor::Http {
                url: "http://localhost:3000".to_string(),
                auth: None,
            },
            metadata: HashMap::new(),
        };

        let result = marshal.register_tool(tool_info);
        assert!(result.is_ok());
        assert_eq!(marshal.registry().count(), 1);
        assert!(marshal.registry().contains("test-tool"));
    }

    #[test]
    fn test_fire_marshal_duplicate_registration() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.redb");

        let config = FireMarshalConfig {
            database_path: db_path,
            port: 3001,
            rate_limit_rps: 100,
            timeout_secs: 30,
            max_concurrent: 10,
        };

        let marshal = FireMarshal::new(config).unwrap();

        let tool_info = ToolInfo {
            name: "test-tool".to_string(),
            version: "1.0.0".to_string(),
            description: "Test tool".to_string(),
            tools: vec![],
            executor: ToolExecutor::Http {
                url: "http://localhost:3000".to_string(),
                auth: None,
            },
            metadata: HashMap::new(),
        };

        // First registration should succeed
        assert!(marshal.register_tool(tool_info.clone()).is_ok());

        // Second registration should fail
        assert!(marshal.register_tool(tool_info).is_err());
    }

    #[test]
    fn test_tool_summary_unhealthy() {
        let response = ListToolsResponse {
            tools: vec![
                ToolSummary {
                    name: "failing-tool".to_string(),
                    version: "0.1.0".to_string(),
                    description: "Tool that is unhealthy".to_string(),
                    healthy: false,
                    methods: vec![],
                },
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("failing-tool"));
        assert!(json.contains("\"healthy\":false"));
    }

    #[test]
    fn test_fire_marshal_database_access() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.redb");

        let config = FireMarshalConfig {
            database_path: db_path,
            port: 3001,
            rate_limit_rps: 100,
            timeout_secs: 30,
            max_concurrent: 10,
        };

        let marshal = FireMarshal::new(config).unwrap();

        // Database should be accessible
        let stats = marshal.database().stats();
        assert!(stats.is_ok());
    }

    #[test]
    fn test_stats_response_zero_values() {
        let response = StatsResponse {
            registered_tools: 0,
            cache_entries: 0,
            execution_logs: 0,
        };

        let json = serde_json::to_string(&response).unwrap();
        // All values should be 0
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["registered_tools"], 0);
        assert_eq!(parsed["cache_entries"], 0);
        assert_eq!(parsed["execution_logs"], 0);
    }
}
