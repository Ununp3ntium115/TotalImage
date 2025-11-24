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

mod protocol;
mod tools;
mod server;
mod cache;

pub use protocol::{
    MCPRequest, MCPResponse, MCPError, MCPErrorCode,
    InitializeParams, CallToolParams, ToolResult, Content,
};
pub use tools::{
    Tool, ToolInfo, ToolEnum,
    AnalyzeDiskImageTool, ListPartitionsTool, ListFilesTool,
    ExtractFileTool, ValidateIntegrityTool,
};
pub use server::{MCPServer, ServerMode, StandaloneConfig, IntegratedConfig};
pub use cache::ToolCache;

pub use totalimage_core::Result as TotalImageResult;
pub use anyhow::Result;
