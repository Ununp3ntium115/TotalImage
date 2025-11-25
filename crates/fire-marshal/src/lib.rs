//! Fire Marshal - Tool Orchestration Framework for PYRO Platform
//!
//! Fire Marshal provides:
//! - Tool registry (static and dynamic registration)
//! - Multiple transport layers (stdio, HTTP, WebSocket)
//! - Shared redb database for cross-tool caching
//! - Rate limiting and resource management
//! - Tool discovery from manifest files

pub mod database;
pub mod error;
pub mod registry;
pub mod server;
pub mod transport;

pub use database::PlatformDatabase;
pub use error::{Error, Result};
pub use registry::{RegisteredTool, ToolExecutor, ToolInfo, ToolRegistry};
pub use server::{FireMarshal, FireMarshalConfig};
