//! Tool registry for managing registered tools
//!
//! Supports multiple executor types:
//! - HTTP: Call tool via HTTP endpoint
//! - Process: Launch tool as subprocess with stdio
//! - Native: In-process Rust function (for embedded tools)

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

/// Tool information for registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Unique tool name
    pub name: String,
    /// Tool version
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Available methods/tools
    pub tools: Vec<ToolMethod>,
    /// How to execute the tool
    pub executor: ToolExecutor,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// A single method/tool exposed by a tool server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMethod {
    /// Method name
    pub name: String,
    /// Method description
    pub description: String,
    /// JSON Schema for input
    pub input_schema: Value,
}

/// How to execute a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolExecutor {
    /// Call tool via HTTP endpoint
    #[serde(rename = "http")]
    Http {
        /// Base URL for the tool
        url: String,
        /// Optional authentication
        #[serde(default)]
        auth: Option<AuthConfig>,
    },

    /// Launch tool as subprocess with stdio
    #[serde(rename = "process")]
    Process {
        /// Path to executable
        executable: PathBuf,
        /// Command line arguments
        #[serde(default)]
        args: Vec<String>,
        /// Environment variables
        #[serde(default)]
        env: HashMap<String, String>,
    },

    /// Tool is embedded in Fire Marshal
    #[serde(rename = "native")]
    Native {
        /// Module identifier
        module: String,
    },
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthConfig {
    /// Bearer token authentication
    #[serde(rename = "bearer")]
    Bearer { token: String },

    /// API key authentication
    #[serde(rename = "api_key")]
    ApiKey {
        header: String,
        key: String,
    },
}

/// A registered tool in the registry
#[derive(Debug, Clone)]
pub struct RegisteredTool {
    /// Tool information
    pub info: ToolInfo,
    /// Registration timestamp
    pub registered_at: u64,
    /// Last health check timestamp
    pub last_health_check: Option<u64>,
    /// Is the tool currently healthy
    pub healthy: bool,
}

/// Tool registry managing all registered tools
pub struct ToolRegistry {
    /// Registered tools by name
    tools: RwLock<HashMap<String, RegisteredTool>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Register a tool
    pub fn register(&self, info: ToolInfo) -> Result<()> {
        let mut tools = self.tools.write().map_err(|_| {
            Error::InvalidConfig("Registry lock poisoned".to_string())
        })?;

        if tools.contains_key(&info.name) {
            return Err(Error::ToolAlreadyRegistered(info.name.clone()));
        }

        let registered = RegisteredTool {
            info: info.clone(),
            registered_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            last_health_check: None,
            healthy: true, // Assume healthy until proven otherwise
        };

        tracing::info!("Registering tool: {} v{}", info.name, info.version);
        tools.insert(info.name.clone(), registered);

        Ok(())
    }

    /// Unregister a tool
    pub fn unregister(&self, name: &str) -> Result<()> {
        let mut tools = self.tools.write().map_err(|_| {
            Error::InvalidConfig("Registry lock poisoned".to_string())
        })?;

        if tools.remove(name).is_none() {
            return Err(Error::ToolNotFound(name.to_string()));
        }

        tracing::info!("Unregistered tool: {}", name);
        Ok(())
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Result<RegisteredTool> {
        let tools = self.tools.read().map_err(|_| {
            Error::InvalidConfig("Registry lock poisoned".to_string())
        })?;

        tools
            .get(name)
            .cloned()
            .ok_or_else(|| Error::ToolNotFound(name.to_string()))
    }

    /// List all registered tools
    pub fn list(&self) -> Result<Vec<RegisteredTool>> {
        let tools = self.tools.read().map_err(|_| {
            Error::InvalidConfig("Registry lock poisoned".to_string())
        })?;

        Ok(tools.values().cloned().collect())
    }

    /// Update tool health status
    pub fn update_health(&self, name: &str, healthy: bool) -> Result<()> {
        let mut tools = self.tools.write().map_err(|_| {
            Error::InvalidConfig("Registry lock poisoned".to_string())
        })?;

        let tool = tools
            .get_mut(name)
            .ok_or_else(|| Error::ToolNotFound(name.to_string()))?;

        tool.healthy = healthy;
        tool.last_health_check = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        Ok(())
    }

    /// Get count of registered tools
    pub fn count(&self) -> usize {
        self.tools
            .read()
            .map(|tools| tools.len())
            .unwrap_or(0)
    }

    /// Check if a tool is registered
    pub fn contains(&self, name: &str) -> bool {
        self.tools
            .read()
            .map(|tools| tools.contains_key(name))
            .unwrap_or(false)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool manifest file format (for discovery)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    /// Tool name
    pub name: String,
    /// Tool version
    pub version: String,
    /// Description
    pub description: String,
    /// Executor configuration
    pub executor: ToolExecutor,
    /// Available tools/methods
    pub tools: Vec<ToolMethod>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ToolManifest {
    /// Load manifest from file
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Convert to ToolInfo
    pub fn into_tool_info(self) -> ToolInfo {
        ToolInfo {
            name: self.name,
            version: self.version,
            description: self.description,
            tools: self.tools,
            executor: self.executor,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_tool() {
        let registry = ToolRegistry::new();

        let info = ToolInfo {
            name: "test-tool".to_string(),
            version: "1.0.0".to_string(),
            description: "A test tool".to_string(),
            tools: vec![],
            executor: ToolExecutor::Http {
                url: "http://localhost:3000".to_string(),
                auth: None,
            },
            metadata: HashMap::new(),
        };

        registry.register(info).unwrap();
        assert!(registry.contains("test-tool"));
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = ToolRegistry::new();

        let info = ToolInfo {
            name: "test-tool".to_string(),
            version: "1.0.0".to_string(),
            description: "A test tool".to_string(),
            tools: vec![],
            executor: ToolExecutor::Http {
                url: "http://localhost:3000".to_string(),
                auth: None,
            },
            metadata: HashMap::new(),
        };

        registry.register(info.clone()).unwrap();
        assert!(registry.register(info).is_err());
    }

    #[test]
    fn test_unregister_tool() {
        let registry = ToolRegistry::new();

        let info = ToolInfo {
            name: "test-tool".to_string(),
            version: "1.0.0".to_string(),
            description: "A test tool".to_string(),
            tools: vec![],
            executor: ToolExecutor::Http {
                url: "http://localhost:3000".to_string(),
                auth: None,
            },
            metadata: HashMap::new(),
        };

        registry.register(info).unwrap();
        registry.unregister("test-tool").unwrap();
        assert!(!registry.contains("test-tool"));
    }
}
