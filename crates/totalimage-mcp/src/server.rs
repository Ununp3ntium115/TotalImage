//! MCP Server implementation with dual-mode operation
//!
//! Supports two modes:
//! - Standalone: stdio transport for Claude Desktop
//! - Integrated: HTTP transport + Fire Marshal registration

use crate::cache::ToolCache;
use crate::protocol::*;
use crate::tools::*;
use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::post,
    Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Server mode configuration
#[derive(Debug, Clone)]
pub enum ServerMode {
    Standalone(StandaloneConfig),
    Integrated(IntegratedConfig),
}

/// Standalone mode configuration (stdio transport)
#[derive(Debug, Clone)]
pub struct StandaloneConfig {
    pub cache_dir: PathBuf,
    pub config_file: Option<PathBuf>,
}

/// Integrated mode configuration (HTTP transport + Fire Marshal)
#[derive(Debug, Clone)]
pub struct IntegratedConfig {
    pub cache_dir: PathBuf,
    pub marshal_url: String,
    pub port: u16,
    pub tool_name: String,
}

/// MCP Server
pub struct MCPServer {
    mode: ServerMode,
    tools: Vec<ToolEnum>,
    cache: Arc<ToolCache>,
}

impl MCPServer {
    /// Create server in standalone mode
    pub fn new_standalone(config: StandaloneConfig) -> Result<Self> {
        let cache_path = config.cache_dir.join("mcp-cache.redb");
        let cache = Arc::new(ToolCache::new(
            cache_path,
            "totalimage-mcp",
            env!("CARGO_PKG_VERSION"),
        )?);

        let tools: Vec<ToolEnum> = vec![
            ToolEnum::AnalyzeDiskImage(AnalyzeDiskImageTool {
                cache: cache.clone(),
            }),
            ToolEnum::ListPartitions(ListPartitionsTool {
                cache: cache.clone(),
            }),
            ToolEnum::ListFiles(ListFilesTool {
                cache: cache.clone(),
            }),
            ToolEnum::ExtractFile(ExtractFileTool {}),
            ToolEnum::ValidateIntegrity(ValidateIntegrityTool {}),
        ];

        Ok(Self {
            mode: ServerMode::Standalone(config),
            tools,
            cache,
        })
    }

    /// Create server in integrated mode
    pub fn new_integrated(config: IntegratedConfig) -> Result<Self> {
        let cache_path = config.cache_dir.join("mcp-cache.redb");
        let cache = Arc::new(ToolCache::new(
            cache_path,
            &config.tool_name,
            env!("CARGO_PKG_VERSION"),
        )?);

        let tools: Vec<ToolEnum> = vec![
            ToolEnum::AnalyzeDiskImage(AnalyzeDiskImageTool {
                cache: cache.clone(),
            }),
            ToolEnum::ListPartitions(ListPartitionsTool {
                cache: cache.clone(),
            }),
            ToolEnum::ListFiles(ListFilesTool {
                cache: cache.clone(),
            }),
            ToolEnum::ExtractFile(ExtractFileTool {}),
            ToolEnum::ValidateIntegrity(ValidateIntegrityTool {}),
        ];

        Ok(Self {
            mode: ServerMode::Integrated(config),
            tools,
            cache,
        })
    }

    /// Check if running in standalone mode
    pub fn is_standalone(&self) -> bool {
        matches!(self.mode, ServerMode::Standalone(_))
    }

    /// Check if running in integrated mode
    pub fn is_integrated(&self) -> bool {
        matches!(self.mode, ServerMode::Integrated(_))
    }

    /// Listen on stdio (standalone mode)
    pub async fn listen_stdio(&self) -> Result<()> {
        tracing::info!("Starting MCP server in stdio mode");

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader
                .read_line(&mut line)
                .await
                .context("Failed to read from stdin")?;

            if bytes_read == 0 {
                // EOF
                break;
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            tracing::debug!("Received request: {}", line);

            // Parse request
            let request: MCPRequest = match serde_json::from_str(line) {
                Ok(req) => req,
                Err(e) => {
                    tracing::error!("Failed to parse request: {}", e);
                    let error_response = MCPResponse {
                        jsonrpc: "2.0".to_string(),
                        id: RequestId::String("error".to_string()),
                        result: None,
                        error: Some(MCPError {
                            code: MCPErrorCode::ParseError,
                            message: format!("Failed to parse request: {}", e),
                            data: None,
                        }),
                    };
                    let response_json = serde_json::to_string(&error_response)?;
                    stdout.write_all(response_json.as_bytes()).await?;
                    stdout.write_all(b"\n").await?;
                    stdout.flush().await?;
                    continue;
                }
            };

            // Handle request
            let response = self.handle_request(request).await;

            // Send response
            let response_json = serde_json::to_string(&response)?;
            tracing::debug!("Sending response: {}", response_json);
            stdout.write_all(response_json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    /// Listen on HTTP (integrated mode)
    pub async fn listen_http(self: Arc<Self>) -> Result<()> {
        let config = match &self.mode {
            ServerMode::Integrated(c) => c,
            _ => anyhow::bail!("listen_http called in standalone mode"),
        };

        tracing::info!("Starting MCP server in HTTP mode on port {}", config.port);

        let app_state = AppState {
            server: self.clone(),
        };

        let app = Router::new()
            .route("/mcp", post(handle_mcp_request))
            .with_state(app_state);

        let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
        let listener = tokio::net::TcpListener::bind(addr).await?;

        tracing::info!("MCP HTTP server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Register with Fire Marshal
    pub async fn register_with_marshal(&self) -> Result<()> {
        let config = match &self.mode {
            ServerMode::Integrated(c) => c,
            _ => anyhow::bail!("register_with_marshal called in standalone mode"),
        };

        tracing::info!("Registering with Fire Marshal at {}", config.marshal_url);

        let client = reqwest::Client::new();
        let registration = json!({
            "name": config.tool_name,
            "version": env!("CARGO_PKG_VERSION"),
            "endpoint": format!("http://127.0.0.1:{}/mcp", config.port),
            "tools": self.tools.iter().map(|t| t.name()).collect::<Vec<_>>(),
        });

        let response = client
            .post(format!("{}/tools/register", config.marshal_url))
            .json(&registration)
            .send()
            .await
            .context("Failed to register with Fire Marshal")?;

        if response.status().is_success() {
            tracing::info!("Successfully registered with Fire Marshal");
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("Fire Marshal registration failed: {} - {}", status, body);
        }
    }

    /// Handle an MCP request
    async fn handle_request(&self, request: MCPRequest) -> MCPResponse {
        match request {
            MCPRequest::Initialize { id, params, .. } => self.handle_initialize(id, params).await,
            MCPRequest::ListTools { id, .. } => self.handle_list_tools(id).await,
            MCPRequest::CallTool { id, params, .. } => self.handle_call_tool(id, params).await,
        }
    }

    async fn handle_initialize(&self, id: RequestId, _params: InitializeParams) -> MCPResponse {
        let result = InitializeResponse {
            protocol_version: MCP_VERSION.to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ServerToolCapabilities {
                    list_changed: None,
                }),
            },
            server_info: ServerInfo {
                name: "totalimage-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        MCPResponse::success(id, serde_json::to_value(result).unwrap())
    }

    async fn handle_list_tools(&self, id: RequestId) -> MCPResponse {
        let tools: Vec<ToolDefinition> = self.tools.iter().map(|t| t.definition()).collect();

        MCPResponse::success(id, json!({ "tools": tools }))
    }

    async fn handle_call_tool(&self, id: RequestId, params: CallToolParams) -> MCPResponse {
        // Find tool
        let tool = match self.tools.iter().find(|t| t.name() == params.name) {
            Some(t) => t,
            None => {
                return MCPResponse::error(id, MCPError::tool_not_found(&params.name));
            }
        };

        // Execute tool
        match tool.execute(params.arguments).await {
            Ok(result) => MCPResponse::success(id, serde_json::to_value(result).unwrap()),
            Err(e) => {
                tracing::error!("Tool execution error: {}", e);
                MCPResponse::error(
                    id,
                    MCPError::internal_error(format!("Tool execution failed: {}", e)),
                )
            }
        }
    }
}

// HTTP handler state
#[derive(Clone)]
struct AppState {
    server: Arc<MCPServer>,
}

// HTTP request handler
async fn handle_mcp_request(
    State(state): State<AppState>,
    Json(request): Json<MCPRequest>,
) -> impl IntoResponse {
    let response = state.server.handle_request(request).await;
    (StatusCode::OK, Json(response))
}
