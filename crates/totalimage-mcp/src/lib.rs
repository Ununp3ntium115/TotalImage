//! TotalImage MCP Server - Model Context Protocol integration for disk image analysis
//!
//! Provides MCP server functionality for TotalImage, enabling integration with:
//! - Claude Desktop (standalone mode via stdio)
//! - Fire Marshal framework (integrated mode via HTTP)
//!
//! # Features
//!
//! - **5 Core Tools**: analyze_disk_image, list_partitions, list_files, extract_file, validate_integrity
//! - **Dual-Mode Operation**: Standalone (stdio) or Integrated (HTTP + Fire Marshal)
//! - **Shared Cache**: redb-based metadata caching with 30-day TTL
//! - **Security Hardening**: Path validation, allocation limits, error sanitization
//!
//! # Usage
//!
//! ## Standalone Mode (Claude Desktop)
//!
//! ```bash
//! totalimage-mcp standalone
//! ```
//!
//! ## Integrated Mode (Fire Marshal)
//!
//! ```bash
//! totalimage-mcp integrated --marshal-url http://localhost:3001 --port 3002
//! ```

mod auth;
mod cache;
pub mod metrics;
mod protocol;
mod server;
mod tools;
mod websocket;

pub use auth::{AuthConfig, AuthError, AuthMethod, AuthUser, Claims};
pub use cache::ToolCache;
pub use metrics::MetricsState;
pub use protocol::{
    CallToolParams, Content, InitializeParams, MCPError, MCPErrorCode, MCPRequest, MCPResponse,
    ToolResult,
};
pub use server::{IntegratedConfig, MCPServer, ServerMode, StandaloneConfig};
pub use tools::{
    AnalyzeDiskImageTool, ExtractFileTool, ListFilesTool, ListPartitionsTool, Tool, ToolEnum,
    ToolInfo, ValidateIntegrityTool,
};
pub use websocket::{
    ws_handler, CompletedUpdate, FailedUpdate, ProgressUpdate, WsMessage, WsState,
};

pub use anyhow::Result;
pub use totalimage_core::Result as TotalImageResult;
