# PYRO Platform Ignition Integration Design

**Status:** Phase 5 Planning
**Version:** 0.1.0
**Date:** 2025-11-24
**Author:** TotalImage Development Team

---

## Executive Summary

This document outlines the integration architecture for TotalImage with the PYRO Platform Ignition framework. The design enables **dual-mode operation**: TotalImage can run as a standalone disk image analysis tool OR integrate seamlessly with the Fire Marshal framework for orchestrated forensic analysis workflows.

### Key Requirements

1. **Standalone Executable** - Self-contained binary with no external dependencies
2. **MCP Server Integration** - Model Context Protocol server for Claude Desktop
3. **Fire Marshal Framework** - Tool orchestration and registry system
4. **redb Database Sharing** - Unified caching across all PYRO tools
5. **Node-RED Integration** - Visual workflow builder nodes
6. **Dual Deployment** - Both standalone AND integrated modes

### Design Principles

- **Zero-Copy Architecture** - Memory-mapped I/O for performance
- **Security-First** - Checked arithmetic, allocation limits, path validation
- **Anarchist Terminology** - Consistent naming (Vault, Territory, Zone, Liberation)
- **Modular Design** - Clear separation between core analysis and integration layers
- **Observable Operations** - Comprehensive logging and metrics

---

## 1. Architecture Overview

### High-Level System Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    PYRO Platform Ignition                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐    │
│  │              Fire Marshal Framework                     │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │    │
│  │  │ Tool Registry│  │  Transports  │  │   Database   │ │    │
│  │  │   - MCP      │  │  - stdio     │  │   - redb     │ │    │
│  │  │   - HTTP     │  │  - HTTP      │  │   - Shared   │ │    │
│  │  │   - Discovery│  │  - WebSocket │  │   - 30d TTL  │ │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘ │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐   │
│  │  TotalImage    │  │  Node-RED      │  │  Ignition      │   │
│  │  MCP Server    │  │  Contrib       │  │  Module        │   │
│  │  - 5 tools     │  │  - 3 nodes     │  │  - Perspective │   │
│  │  - Dual mode   │  │  - REST bridge │  │  - Scripting   │   │
│  └────────────────┘  └────────────────┘  └────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Uses
                              ▼
         ┌─────────────────────────────────────────┐
         │         TotalImage Core Library          │
         │  - totalimage-core (traits, errors)     │
         │  - totalimage-pipeline (I/O)            │
         │  - totalimage-vaults (VHD, Raw)         │
         │  - totalimage-zones (MBR, GPT)          │
         │  - totalimage-territories (FAT, ISO)    │
         └─────────────────────────────────────────┘
```

### Deployment Modes

#### Mode 1: Standalone Execution
```
User → totalimage CLI → TotalImage Core → Disk Images
```

#### Mode 2: MCP Server (Claude Desktop)
```
Claude Desktop → MCP Protocol (stdio) → TotalImage MCP Server → TotalImage Core
```

#### Mode 3: Fire Marshal Integration
```
Node-RED/Ignition → HTTP → Fire Marshal → Tool Registry → TotalImage MCP Server
                                             │
                                             └→ Shared redb Cache
```

---

## 2. Component Design

### 2.1 TotalImage MCP Server

**Location:** `packages/totalimage-mcp/`
**Language:** Rust
**Crate Type:** Binary + Library

#### Tool Definitions

```rust
// packages/totalimage-mcp/src/tools.rs

pub enum TotalImageTool {
    AnalyzeDiskImage,
    ListPartitions,
    ListFiles,
    ExtractFile,
    ValidateIntegrity,
}

#[derive(Serialize, Deserialize)]
pub struct AnalyzeDiskImageInput {
    pub path: String,
    #[serde(default = "default_true")]
    pub cache: bool,
    #[serde(default)]
    pub deep_scan: bool,
}

#[derive(Serialize, Deserialize)]
pub struct AnalyzeDiskImageOutput {
    pub vault: VaultInfo,
    pub zones: Vec<ZoneInfo>,
    pub filesystems: Vec<FilesystemInfo>,
    pub security: SecurityAnalysis,
}

pub struct SecurityAnalysis {
    pub boot_sector_valid: bool,
    pub partition_table_valid: bool,
    pub checksum_results: Vec<ChecksumResult>,
    pub suspicious_patterns: Vec<SuspiciousPattern>,
}

impl Tool for AnalyzeDiskImageTool {
    fn name(&self) -> &str {
        "analyze_disk_image"
    }

    fn description(&self) -> &str {
        "Comprehensive disk image analysis: vault type, partitions, filesystems, security validation"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to disk image file (.img, .vhd, .iso)"
                },
                "cache": {
                    "type": "boolean",
                    "default": true,
                    "description": "Use cached results if available"
                },
                "deep_scan": {
                    "type": "boolean",
                    "default": false,
                    "description": "Perform deep filesystem scan (slower)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let input: AnalyzeDiskImageInput = serde_json::from_value(args)?;

        // Validate path (prevent path traversal)
        let path = validate_file_path(&input.path)?;

        // Check cache
        if input.cache {
            if let Some(cached) = self.cache.get::<AnalyzeDiskImageOutput>(&input.path)? {
                return Ok(ToolResult::success(cached));
            }
        }

        // Perform analysis
        let vault_info = analyze_vault(&path)?;
        let zone_info = analyze_zones(&path)?;
        let filesystem_info = if input.deep_scan {
            analyze_filesystems_deep(&path)?
        } else {
            analyze_filesystems_quick(&path)?
        };
        let security = validate_security(&path)?;

        let output = AnalyzeDiskImageOutput {
            vault: vault_info,
            zones: zone_info,
            filesystems: filesystem_info,
            security,
        };

        // Cache result
        if input.cache {
            self.cache.set(&input.path, &output)?;
        }

        Ok(ToolResult::success(output))
    }
}
```

#### Dual-Mode Operation

```rust
// packages/totalimage-mcp/src/main.rs

use clap::{Parser, Subcommand};
use totalimage_mcp::{MCPServer, StandaloneConfig, IntegratedConfig};

#[derive(Parser)]
#[command(name = "totalimage-mcp")]
#[command(about = "TotalImage MCP Server - Disk Image Analysis for Claude")]
struct Cli {
    #[command(subcommand)]
    mode: Option<Mode>,

    /// Cache directory
    #[arg(long, env = "TOTALIMAGE_CACHE_DIR")]
    cache_dir: Option<PathBuf>,

    /// Log level
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Mode {
    /// Standalone mode (stdio transport for Claude Desktop)
    Standalone {
        /// Configuration file
        #[arg(long)]
        config: Option<PathBuf>,
    },

    /// Integrated mode (HTTP transport + Fire Marshal registration)
    Integrated {
        /// Fire Marshal URL
        #[arg(long, env = "FIRE_MARSHAL_URL")]
        marshal_url: String,

        /// HTTP server port
        #[arg(long, default_value = "3002")]
        port: u16,

        /// Tool name for registry
        #[arg(long, default_value = "totalimage")]
        tool_name: String,
    },

    /// Auto-detect mode based on environment
    Auto,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(cli.log_level)
        .init();

    // Determine cache directory
    let cache_dir = cli.cache_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(format!("{}/.cache/totalimage", home))
    });

    // Run in appropriate mode
    match cli.mode.unwrap_or(Mode::Auto) {
        Mode::Standalone { config } => {
            tracing::info!("Starting TotalImage MCP Server in STANDALONE mode");
            let config = StandaloneConfig {
                cache_dir,
                config_file: config,
            };
            let server = MCPServer::new_standalone(config)?;
            server.listen_stdio().await?;
        }

        Mode::Integrated { marshal_url, port, tool_name } => {
            tracing::info!("Starting TotalImage MCP Server in INTEGRATED mode");
            tracing::info!("  Fire Marshal: {}", marshal_url);
            tracing::info!("  HTTP Port: {}", port);
            tracing::info!("  Tool Name: {}", tool_name);

            let config = IntegratedConfig {
                cache_dir,
                marshal_url,
                port,
                tool_name,
            };
            let server = MCPServer::new_integrated(config)?;

            // Register with Fire Marshal
            server.register_with_marshal().await?;

            // Start HTTP server
            server.listen_http().await?;
        }

        Mode::Auto => {
            // Auto-detect based on environment
            if let Ok(marshal_url) = std::env::var("FIRE_MARSHAL_URL") {
                tracing::info!("Auto-detected INTEGRATED mode (FIRE_MARSHAL_URL set)");
                let config = IntegratedConfig {
                    cache_dir,
                    marshal_url,
                    port: 3002,
                    tool_name: "totalimage".to_string(),
                };
                let server = MCPServer::new_integrated(config)?;
                server.register_with_marshal().await?;
                server.listen_http().await?;
            } else {
                tracing::info!("Auto-detected STANDALONE mode");
                let config = StandaloneConfig {
                    cache_dir,
                    config_file: None,
                };
                let server = MCPServer::new_standalone(config)?;
                server.listen_stdio().await?;
            }
        }
    }

    Ok(())
}
```

#### MCP Protocol Implementation

```rust
// packages/totalimage-mcp/src/protocol.rs

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum MCPRequest {
    #[serde(rename = "initialize")]
    Initialize {
        id: String,
        params: InitializeParams,
    },

    #[serde(rename = "tools/list")]
    ListTools {
        id: String,
    },

    #[serde(rename = "tools/call")]
    CallTool {
        id: String,
        params: CallToolParams,
    },
}

#[derive(Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: Capabilities,
    pub client_info: ClientInfo,
}

#[derive(Serialize, Deserialize)]
pub struct MCPResponse {
    pub id: String,
    pub result: Option<Value>,
    pub error: Option<MCPError>,
}

pub struct MCPServer {
    tools: Vec<Box<dyn Tool>>,
    cache: Arc<MetadataCache>,
    config: ServerConfig,
}

impl MCPServer {
    pub async fn handle_request(&self, request: MCPRequest) -> MCPResponse {
        match request {
            MCPRequest::Initialize { id, params } => {
                self.handle_initialize(id, params).await
            }
            MCPRequest::ListTools { id } => {
                self.handle_list_tools(id).await
            }
            MCPRequest::CallTool { id, params } => {
                self.handle_call_tool(id, params).await
            }
        }
    }

    async fn handle_call_tool(&self, id: String, params: CallToolParams) -> MCPResponse {
        // Find tool
        let tool = self.tools.iter()
            .find(|t| t.name() == params.name)
            .ok_or_else(|| MCPError::tool_not_found(&params.name));

        match tool {
            Ok(tool) => {
                // Execute tool
                match tool.execute(params.arguments).await {
                    Ok(result) => MCPResponse {
                        id,
                        result: Some(json!(result)),
                        error: None,
                    },
                    Err(e) => MCPResponse {
                        id,
                        result: None,
                        error: Some(MCPError::execution_error(&e)),
                    },
                }
            }
            Err(e) => MCPResponse {
                id,
                result: None,
                error: Some(e),
            },
        }
    }
}
```

---

### 2.2 Fire Marshal Framework

**Location:** `packages/fire-marshal/`
**Language:** Rust
**Purpose:** Tool orchestration and registry for PYRO Platform

#### Core Architecture

```rust
// packages/fire-marshal/src/lib.rs

pub struct FireMarshal {
    registry: Arc<RwLock<ToolRegistry>>,
    database: Arc<Mutex<Database>>,
    transports: Vec<Box<dyn Transport>>,
    config: FireMarshalConfig,
}

impl FireMarshal {
    pub async fn new(config: FireMarshalConfig) -> Result<Self> {
        // Initialize shared redb database
        let db_path = config.database_path.clone();
        let database = Database::create(&db_path)?;

        // Create tool registry
        let registry = ToolRegistry::new();

        // Initialize transports
        let transports = vec![
            Box::new(StdioTransport::new()) as Box<dyn Transport>,
            Box::new(HttpTransport::new(config.http_port)) as Box<dyn Transport>,
        ];

        Ok(Self {
            registry: Arc::new(RwLock::new(registry)),
            database: Arc::new(Mutex::new(database)),
            transports,
            config,
        })
    }

    pub async fn discover_tools(&self, search_paths: Vec<PathBuf>) -> Result<usize> {
        let mut registry = self.registry.write().await;
        let mut discovered = 0;

        for path in search_paths {
            for entry in walkdir::WalkDir::new(path) {
                let entry = entry?;
                if entry.file_name().to_string_lossy().ends_with(".tool.json") {
                    let manifest = ToolManifest::load(entry.path())?;
                    registry.register_from_manifest(manifest)?;
                    discovered += 1;
                }
            }
        }

        tracing::info!("Discovered {} tools", discovered);
        Ok(discovered)
    }

    pub async fn register_tool_runtime(&self, tool_info: ToolInfo) -> Result<()> {
        let mut registry = self.registry.write().await;
        registry.register_runtime(tool_info)?;

        // Persist to database
        let db = self.database.lock().unwrap();
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(TOOL_REGISTRY_TABLE)?;
            let encoded = bincode::serialize(&tool_info)?;
            table.insert(tool_info.name.as_str(), encoded.as_slice())?;
        }
        write_txn.commit()?;

        tracing::info!("Registered tool: {}", tool_info.name);
        Ok(())
    }

    pub async fn call_tool(&self, tool_name: &str, args: Value) -> Result<ToolResult> {
        let registry = self.registry.read().await;
        let tool = registry.get(tool_name)?;

        // Execute tool
        let result = tool.execute(args).await?;

        // Store execution in database (audit trail)
        self.log_execution(tool_name, &result).await?;

        Ok(result)
    }
}
```

#### Tool Registry

```rust
// packages/fire-marshal/src/registry.rs

pub struct ToolRegistry {
    tools: HashMap<String, RegisteredTool>,
}

pub struct RegisteredTool {
    pub name: String,
    pub description: String,
    pub version: String,
    pub schema: Value,
    pub executor: ToolExecutor,
    pub metadata: ToolMetadata,
}

pub enum ToolExecutor {
    /// In-process Rust function
    Native(Box<dyn Tool>),

    /// External process via stdio
    Process {
        executable: PathBuf,
        args: Vec<String>,
    },

    /// HTTP endpoint
    Http {
        url: String,
        auth: Option<AuthConfig>,
    },

    /// Dynamic library (.so, .dll)
    DynamicLib {
        library_path: PathBuf,
        symbol_name: String,
    },
}

impl ToolRegistry {
    pub fn register_from_manifest(&mut self, manifest: ToolManifest) -> Result<()> {
        let executor = match manifest.executor_type {
            ExecutorType::Process => ToolExecutor::Process {
                executable: manifest.executable.unwrap(),
                args: manifest.args.unwrap_or_default(),
            },
            ExecutorType::Http => ToolExecutor::Http {
                url: manifest.url.unwrap(),
                auth: manifest.auth,
            },
            ExecutorType::DynamicLib => ToolExecutor::DynamicLib {
                library_path: manifest.library_path.unwrap(),
                symbol_name: manifest.symbol_name.unwrap_or_else(|| "register_tools".to_string()),
            },
        };

        let tool = RegisteredTool {
            name: manifest.name,
            description: manifest.description,
            version: manifest.version,
            schema: manifest.schema,
            executor,
            metadata: manifest.metadata,
        };

        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }
}
```

#### Tool Manifest Format

```json
// example: ~/.pyro/tools/totalimage.tool.json
{
  "name": "totalimage",
  "version": "0.1.0",
  "description": "Disk image analysis tool with FAT, ISO, MBR, GPT support",
  "executor_type": "process",
  "executable": "/usr/local/bin/totalimage-mcp",
  "args": ["integrated", "--marshal-url", "http://localhost:3001"],
  "tools": [
    {
      "name": "analyze_disk_image",
      "description": "Comprehensive disk image analysis",
      "schema": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "cache": { "type": "boolean", "default": true },
          "deep_scan": { "type": "boolean", "default": false }
        },
        "required": ["path"]
      }
    },
    {
      "name": "extract_file",
      "description": "Extract file from disk image",
      "schema": {
        "type": "object",
        "properties": {
          "image_path": { "type": "string" },
          "file_path": { "type": "string" },
          "zone_index": { "type": "number" },
          "output_path": { "type": "string" }
        },
        "required": ["image_path", "file_path", "zone_index"]
      }
    }
  ],
  "dependencies": {
    "redb": ">=2.1"
  },
  "metadata": {
    "author": "TotalImage Team",
    "license": "GPL-3.0",
    "repository": "https://github.com/Ununp3ntium115/TotalImage"
  }
}
```

---

### 2.3 Shared redb Database

**Purpose:** Unified caching across all PYRO Platform tools

#### Schema Design

```rust
// packages/fire-marshal/src/database.rs

use redb::TableDefinition;

// Table definitions
const VAULT_INFO_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("vault_info");
const ZONE_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("zone_tables");
const FILESYSTEM_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("filesystems");
const TOOL_REGISTRY_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("tool_registry");
const EXECUTION_LOG_TABLE: TableDefinition<&str, &[u8]> =
    TableDefinition::new("execution_log");

// Cache entry with metadata
#[derive(Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub created_at: u64,  // Unix timestamp
    pub tool: String,      // Which tool created this entry
    pub version: String,   // Tool version for cache invalidation
}

impl<T: Serialize> CacheEntry<T> {
    pub fn new(data: T, tool: &str, version: &str) -> Self {
        Self {
            data,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tool: tool.to_string(),
            version: version.to_string(),
        }
    }

    pub fn is_expired(&self, ttl_seconds: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.created_at > ttl_seconds
    }
}

pub struct PlatformDatabase {
    db: Arc<Mutex<Database>>,
    config: DatabaseConfig,
}

impl PlatformDatabase {
    pub fn new(db_path: PathBuf, config: DatabaseConfig) -> Result<Self> {
        let db = Database::create(&db_path)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(VAULT_INFO_TABLE)?;
            write_txn.open_table(ZONE_TABLE)?;
            write_txn.open_table(FILESYSTEM_TABLE)?;
            write_txn.open_table(TOOL_REGISTRY_TABLE)?;
            write_txn.open_table(EXECUTION_LOG_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            config,
        })
    }

    pub fn set<T: Serialize>(
        &self,
        table: TableDefinition<&str, &[u8]>,
        key: &str,
        value: &T,
        tool: &str,
        version: &str,
    ) -> Result<()> {
        let entry = CacheEntry::new(value, tool, version);
        let encoded = bincode::serialize(&entry)?;

        let db = self.db.lock().unwrap();
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(table)?;
            table.insert(key, encoded.as_slice())?;
        }
        write_txn.commit()?;

        Ok(())
    }

    pub fn get<T: DeserializeOwned>(
        &self,
        table: TableDefinition<&str, &[u8]>,
        key: &str,
    ) -> Result<Option<T>> {
        let db = self.db.lock().unwrap();
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(table)?;

        match table.get(key)? {
            Some(value) => {
                let entry: CacheEntry<T> = bincode::deserialize(value.value())?;

                // Check expiration
                if entry.is_expired(self.config.ttl_seconds) {
                    drop(read_txn);
                    // Entry expired, remove it
                    let write_txn = db.begin_write()?;
                    {
                        let mut table = write_txn.open_table(table)?;
                        table.remove(key)?;
                    }
                    write_txn.commit()?;
                    return Ok(None);
                }

                Ok(Some(entry.data))
            }
            None => Ok(None),
        }
    }
}
```

---

### 2.4 Node-RED Integration

**Location:** `packages/node-red-contrib-pyro/`
**Language:** JavaScript (Node.js)
**Purpose:** Visual workflow integration for forensic analysis

#### Node Definitions

```javascript
// packages/node-red-contrib-pyro/nodes/totalimage.js

module.exports = function(RED) {
    // Analyze Disk Image Node
    function TotalImageAnalyzeNode(config) {
        RED.nodes.createNode(this, config);
        var node = this;

        node.on('input', async function(msg) {
            const imagePath = msg.payload.path || config.path;
            const cache = msg.payload.cache !== undefined ? msg.payload.cache : true;
            const deepScan = msg.payload.deep_scan || false;

            try {
                // Call Fire Marshal or TotalImage directly
                const fireMarshallUrl = process.env.FIRE_MARSHAL_URL || 'http://localhost:3001';

                const response = await fetch(`${fireMarshallUrl}/tools/call`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        tool: 'totalimage',
                        method: 'analyze_disk_image',
                        arguments: {
                            path: imagePath,
                            cache: cache,
                            deep_scan: deepScan
                        }
                    })
                });

                const result = await response.json();

                msg.payload = {
                    vault: result.vault,
                    zones: result.zones,
                    filesystems: result.filesystems,
                    security: result.security
                };

                node.status({ fill: "green", shape: "dot", text: "analyzed" });
                node.send(msg);

            } catch (err) {
                node.error(err, msg);
                node.status({ fill: "red", shape: "ring", text: "error" });
            }
        });
    }
    RED.nodes.registerType("totalimage-analyze", TotalImageAnalyzeNode);

    // Extract File Node
    function TotalImageExtractNode(config) {
        RED.nodes.createNode(this, config);
        var node = this;

        node.on('input', async function(msg) {
            const imagePath = msg.payload.image_path || config.image_path;
            const filePath = msg.payload.file_path || config.file_path;
            const zoneIndex = msg.payload.zone_index || config.zone_index || 0;
            const outputPath = msg.payload.output_path || config.output_path;

            try {
                const fireMarshallUrl = process.env.FIRE_MARSHAL_URL || 'http://localhost:3001';

                const response = await fetch(`${fireMarshallUrl}/tools/call`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        tool: 'totalimage',
                        method: 'extract_file',
                        arguments: {
                            image_path: imagePath,
                            file_path: filePath,
                            zone_index: zoneIndex,
                            output_path: outputPath
                        }
                    })
                });

                const result = await response.json();

                msg.payload = {
                    success: result.success,
                    bytes_extracted: result.bytes_extracted,
                    output_path: result.output_path
                };

                node.status({ fill: "green", shape: "dot", text: "extracted" });
                node.send(msg);

            } catch (err) {
                node.error(err, msg);
                node.status({ fill: "red", shape: "ring", text: "error" });
            }
        });
    }
    RED.nodes.registerType("totalimage-extract", TotalImageExtractNode);
};
```

#### Node-RED UI Definition

```html
<!-- packages/node-red-contrib-pyro/nodes/totalimage.html -->

<script type="text/javascript">
    RED.nodes.registerType('totalimage-analyze', {
        category: 'PYRO Forensics',
        color: '#FFA500',
        defaults: {
            name: { value: "" },
            path: { value: "" },
            cache: { value: true },
            deep_scan: { value: false }
        },
        inputs: 1,
        outputs: 1,
        icon: "font-awesome/fa-database",
        label: function() {
            return this.name || "Analyze Disk Image";
        },
        paletteLabel: "analyze image"
    });
</script>

<script type="text/html" data-template-name="totalimage-analyze">
    <div class="form-row">
        <label for="node-input-name"><i class="fa fa-tag"></i> Name</label>
        <input type="text" id="node-input-name" placeholder="Name">
    </div>
    <div class="form-row">
        <label for="node-input-path"><i class="fa fa-file"></i> Image Path</label>
        <input type="text" id="node-input-path" placeholder="/path/to/disk.img">
    </div>
    <div class="form-row">
        <label for="node-input-cache"><i class="fa fa-database"></i> Use Cache</label>
        <input type="checkbox" id="node-input-cache" checked>
    </div>
    <div class="form-row">
        <label for="node-input-deep_scan"><i class="fa fa-search"></i> Deep Scan</label>
        <input type="checkbox" id="node-input-deep_scan">
    </div>
</script>

<script type="text/html" data-help-name="totalimage-analyze">
    <p>Analyzes a disk image and returns vault, partition, and filesystem information.</p>
    <h3>Inputs</h3>
    <dl class="message-properties">
        <dt>payload.path <span class="property-type">string</span></dt>
        <dd>Path to the disk image file</dd>
        <dt>payload.cache <span class="property-type">boolean</span></dt>
        <dd>Whether to use cached results (default: true)</dd>
        <dt>payload.deep_scan <span class="property-type">boolean</span></dt>
        <dd>Perform deep filesystem scan (default: false)</dd>
    </dl>
    <h3>Outputs</h3>
    <dl class="message-properties">
        <dt>payload.vault <span class="property-type">object</span></dt>
        <dd>Container format information (type, size)</dd>
        <dt>payload.zones <span class="property-type">array</span></dt>
        <dd>List of partitions/zones</dd>
        <dt>payload.filesystems <span class="property-type">array</span></dt>
        <dd>Filesystem metadata</dd>
        <dt>payload.security <span class="property-type">object</span></dt>
        <dd>Security validation results</dd>
    </dl>
</script>
```

---

## 3. Deployment Strategy

### 3.1 Standalone Packaging

#### Linux (x86_64)

```bash
# Build static binary with musl
cargo build --target x86_64-unknown-linux-musl --release -p totalimage-mcp

# Package with dependencies
mkdir -p totalimage-standalone
cp target/x86_64-unknown-linux-musl/release/totalimage-mcp totalimage-standalone/
cp .mcp-config.json totalimage-standalone/
cp README.md totalimage-standalone/
tar -czf totalimage-mcp-v0.1.0-linux-x86_64.tar.gz totalimage-standalone/
```

#### macOS (Universal Binary)

```bash
# Build for both architectures
cargo build --target x86_64-apple-darwin --release -p totalimage-mcp
cargo build --target aarch64-apple-darwin --release -p totalimage-mcp

# Create universal binary
lipo -create \
    target/x86_64-apple-darwin/release/totalimage-mcp \
    target/aarch64-apple-darwin/release/totalimage-mcp \
    -output totalimage-mcp-universal

# Package as .dmg or .pkg
# (requires additional tooling like create-dmg)
```

#### Windows (x86_64)

```bash
# Cross-compile from Linux (or build natively on Windows)
cargo build --target x86_64-pc-windows-gnu --release -p totalimage-mcp

# Package with installer (optional - use NSIS or WiX)
```

### 3.2 Docker Deployment

```dockerfile
# Dockerfile.standalone
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p totalimage-mcp

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/totalimage-mcp /usr/local/bin/
COPY .mcp-config.json /etc/totalimage/mcp-config.json

ENV TOTALIMAGE_CACHE_DIR=/var/cache/totalimage
RUN mkdir -p /var/cache/totalimage

EXPOSE 3002
CMD ["totalimage-mcp", "standalone"]
```

```dockerfile
# Dockerfile.integrated
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p totalimage-mcp -p fire-marshal

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/totalimage-mcp /usr/local/bin/
COPY --from=builder /app/target/release/fire-marshal /usr/local/bin/
COPY configs/ /etc/pyro/

ENV FIRE_MARSHAL_URL=http://localhost:3001
ENV TOTALIMAGE_CACHE_DIR=/var/cache/totalimage
RUN mkdir -p /var/cache/totalimage

EXPOSE 3001 3002
CMD ["fire-marshal", "start"]
```

### 3.3 Claude Desktop Configuration

```json
// ~/.config/Claude/claude_desktop_config.json
{
  "mcpServers": {
    "totalimage": {
      "command": "/usr/local/bin/totalimage-mcp",
      "args": ["standalone"],
      "env": {
        "TOTALIMAGE_CACHE_DIR": "/Users/user/.cache/totalimage",
        "RUST_LOG": "info"
      }
    }
  }
}
```

---

## 4. Implementation Roadmap

### Phase 5.1: MCP Server Foundation (Week 1-2)

**Goal:** Create functional MCP server for TotalImage

1. Create `packages/totalimage-mcp/` crate
2. Implement 5 core tools:
   - `analyze_disk_image`
   - `list_partitions`
   - `list_files`
   - `extract_file`
   - `validate_integrity`
3. Implement stdio transport for Claude Desktop
4. Test with Claude Desktop integration
5. Write comprehensive documentation

**Deliverables:**
- ✅ Working MCP server binary
- ✅ Claude Desktop integration tested
- ✅ 5 tools fully functional
- ✅ Documentation + examples

### Phase 5.2: Fire Marshal Framework (Week 3-4)

**Goal:** Build tool orchestration framework

1. Create `packages/fire-marshal/` crate
2. Implement tool registry (static + dynamic)
3. Build HTTP transport layer
4. Create shared redb database with cross-tool schema
5. Implement tool discovery (`.tool.json` manifests)
6. Test with TotalImage + mock secondary tool

**Deliverables:**
- ✅ Fire Marshal framework operational
- ✅ Tool registry working (static + manifest-based)
- ✅ HTTP transport functional
- ✅ Shared database tested
- ✅ Documentation

### Phase 5.3: Dual-Mode Integration (Week 5)

**Goal:** Enable TotalImage to work standalone AND integrated

1. Refactor TotalImage MCP to support dual modes
2. Implement auto-detection logic
3. Add Fire Marshal registration endpoint
4. Test standalone mode (Claude Desktop)
5. Test integrated mode (Fire Marshal)
6. Performance benchmarking

**Deliverables:**
- ✅ Dual-mode binary working
- ✅ Auto-detection functional
- ✅ Both modes tested
- ✅ Performance metrics documented

### Phase 5.4: Node-RED Integration (Week 6)

**Goal:** Create Node-RED contrib package

1. Create `packages/node-red-contrib-pyro/`
2. Implement 3 nodes:
   - `totalimage-analyze`
   - `totalimage-extract`
   - `totalimage-list`
3. Build UI components (HTML)
4. Test with Fire Marshal integration
5. Create example flows
6. Publish to npm

**Deliverables:**
- ✅ node-red-contrib-pyro package published
- ✅ 3 nodes functional
- ✅ Example flows documented
- ✅ README with screenshots

### Phase 5.5: Documentation & Deployment (Week 7)

**Goal:** Production-ready deployment

1. Write comprehensive architecture docs
2. Create Docker images (standalone + integrated)
3. Build GitHub Actions CI/CD pipelines
4. Package releases (Linux, macOS, Windows)
5. Write deployment guides
6. Create video tutorials

**Deliverables:**
- ✅ ARCHITECTURE.md
- ✅ DEPLOYMENT.md
- ✅ Docker images published
- ✅ Binary releases on GitHub
- ✅ Video tutorials

### Phase 5.6: Production Hardening (Week 8)

**Goal:** Address SEC-007 and production concerns

1. Implement rate limiting (tower middleware)
2. Add request timeouts (30s)
3. Add concurrency limits (10 concurrent)
4. Configure CORS policies
5. Implement TLS/HTTPS support
6. Add monitoring/metrics (Prometheus)
7. Load testing

**Deliverables:**
- ✅ SEC-007 fully mitigated
- ✅ Rate limiting tested
- ✅ TLS configured
- ✅ Metrics dashboard
- ✅ Load test results

---

## 5. Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_disk_image_tool() {
        let cache = Arc::new(MetadataCache::new_temp().unwrap());
        let tool = AnalyzeDiskImageTool { cache };

        let args = json!({
            "path": "tests/fixtures/floppy.img",
            "cache": false,
            "deep_scan": false
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.is_success());

        let output: AnalyzeDiskImageOutput = serde_json::from_value(result.content[0].clone()).unwrap();
        assert_eq!(output.vault.vault_type, "Raw Sector Image");
    }

    #[tokio::test]
    async fn test_dual_mode_standalone() {
        let config = StandaloneConfig {
            cache_dir: PathBuf::from("/tmp/test-cache"),
            config_file: None,
        };

        let server = MCPServer::new_standalone(config).unwrap();
        assert!(server.is_standalone());
    }

    #[tokio::test]
    async fn test_dual_mode_integrated() {
        let config = IntegratedConfig {
            cache_dir: PathBuf::from("/tmp/test-cache"),
            marshal_url: "http://localhost:3001".to_string(),
            port: 3002,
            tool_name: "totalimage-test".to_string(),
        };

        let server = MCPServer::new_integrated(config).unwrap();
        assert!(server.is_integrated());
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_fire_marshal_tool_registry() {
    let marshal = FireMarshal::new(test_config()).await.unwrap();

    // Register TotalImage
    let tool_info = ToolInfo {
        name: "totalimage".to_string(),
        version: "0.1.0".to_string(),
        endpoint: "http://localhost:3002".to_string(),
        tools: vec!["analyze_disk_image".to_string()],
    };

    marshal.register_tool_runtime(tool_info).await.unwrap();

    // Call tool
    let result = marshal.call_tool("totalimage", "analyze_disk_image", json!({
        "path": "tests/fixtures/floppy.img"
    })).await.unwrap();

    assert!(result.is_success());
}
```

### End-to-End Tests

```bash
#!/bin/bash
# tests/e2e/test_standalone.sh

# Start TotalImage MCP in standalone mode
./target/release/totalimage-mcp standalone &
PID=$!

# Wait for server to start
sleep 2

# Test via stdio
echo '{"method":"initialize","id":"1","params":{"protocol_version":"1.0"}}' | \
    ./target/release/totalimage-mcp standalone

# Cleanup
kill $PID
```

---

## 6. Security Considerations

### Additional Hardening for MCP Server

1. **Input Validation**
   - All file paths validated with `validate_file_path()`
   - Path traversal prevention (already implemented)
   - Size limits enforced

2. **Resource Limits**
   - Max concurrent tool executions: 10
   - Request timeout: 30 seconds
   - Max request size: 10 MB
   - Memory limits: 256 MB per operation

3. **Authentication (Integrated Mode)**
   - API key authentication for Fire Marshal registration
   - TLS/HTTPS for all HTTP transport
   - CORS policy for browser access

4. **Audit Logging**
   - All tool executions logged to redb
   - Includes: timestamp, user, tool, args, result
   - Retention: 90 days

---

## 7. Performance Targets

### Latency

- **analyze_disk_image** (1.44 MB floppy): < 50ms (cached), < 200ms (uncached)
- **analyze_disk_image** (10 GB VHD): < 500ms (cached), < 2s (uncached)
- **extract_file** (1 MB file): < 100ms
- **list_files** (< 1000 files): < 50ms

### Throughput

- **Concurrent tool executions**: 10 simultaneous
- **HTTP requests/sec**: 100+ (integrated mode)
- **MCP messages/sec**: 50+ (stdio mode)

### Memory

- **Baseline memory**: < 50 MB
- **Per-operation memory**: < 256 MB
- **Cache size**: < 100 MB (configurable)

---

## 8. Future Enhancements

### Phase 6 (Future)

- **Additional Filesystems**: NTFS, ext2/3/4, APFS
- **Write Support**: Modify disk images (currently read-only)
- **Ignition Module**: SCADA integration with Perspective views
- **Advanced Analysis**:
  - File carving for deleted files
  - Entropy analysis for encrypted regions
  - Timeline generation
  - Hash database integration (NSRL, VirusTotal)
- **Distributed Analysis**: Split large images across multiple workers
- **Web UI**: Browser-based interface (alternative to Node-RED)

---

## Appendix A: File Structure

```
/home/user/PYRO_Platform_Ignition/
├── packages/
│   ├── fire-marshal/                     # Rust crate
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                   # Public API
│   │   │   ├── server.rs                # Fire Marshal server
│   │   │   ├── registry.rs              # Tool registry
│   │   │   ├── database.rs              # redb integration
│   │   │   ├── transport/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── stdio.rs
│   │   │   │   ├── http.rs
│   │   │   │   └── websocket.rs
│   │   │   └── integration/
│   │   │       ├── mod.rs
│   │   │       └── node_red.rs
│   │   ├── tests/
│   │   └── README.md
│   │
│   ├── totalimage-mcp/                   # Rust crate (binary)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs                  # Dual-mode binary
│   │   │   ├── lib.rs                   # Library API
│   │   │   ├── tools.rs                 # 5 tool implementations
│   │   │   ├── protocol.rs              # MCP protocol
│   │   │   └── handlers.rs              # Request handlers
│   │   ├── tests/
│   │   ├── .mcp-config.json             # Config template
│   │   └── README.md
│   │
│   └── node-red-contrib-pyro/            # NPM package
│       ├── package.json
│       ├── nodes/
│       │   ├── totalimage.js            # Node implementations
│       │   ├── totalimage.html          # UI definitions
│       │   └── fire-marshal.js          # Fire Marshal connector
│       ├── examples/
│       │   └── forensic-workflow.json   # Example flow
│       └── README.md
│
├── docs/
│   ├── ARCHITECTURE.md                   # System architecture
│   ├── FIRE-MARSHAL.md                   # Framework docs
│   ├── TOTALIMAGE-INTEGRATION.md         # Integration guide
│   ├── DEPLOYMENT.md                     # Deployment guide
│   └── API.md                            # API reference
│
├── examples/
│   ├── standalone-mcp/                   # Standalone examples
│   │   ├── claude-config.json
│   │   └── test-analysis.sh
│   ├── node-red-flows/                   # Node-RED examples
│   │   ├── basic-analysis.json
│   │   └── forensic-pipeline.json
│   └── integrated/                       # Integrated examples
│       └── multi-tool-workflow.json
│
├── .github/
│   └── workflows/
│       ├── ci.yml                        # CI/CD pipeline
│       ├── release.yml                   # Release automation
│       └── docker.yml                    # Docker builds
│
├── Cargo.toml                            # Rust workspace
├── package.json                          # NPM workspace
├── turbo.json                            # Build orchestration
├── Dockerfile.standalone                 # Standalone Docker
├── Dockerfile.integrated                 # Integrated Docker
├── LICENSE                               # GPL-3.0
└── README.md                             # Project overview
```

---

## Appendix B: API Reference

### Fire Marshal HTTP API

```
POST /tools/register
  Request: { "name": "...", "version": "...", "endpoint": "...", "tools": [...] }
  Response: { "success": true, "tool_id": "..." }

GET /tools/list
  Response: { "tools": [{"name": "...", "version": "...", ...}] }

POST /tools/call
  Request: { "tool": "...", "method": "...", "arguments": {...} }
  Response: { "result": {...}, "is_error": false }

GET /database/stats
  Response: { "tables": [...], "size_bytes": ..., "entry_count": ... }

POST /database/cleanup
  Response: { "deleted_entries": ..., "reclaimed_bytes": ... }
```

### TotalImage MCP Tools

```
Tool: analyze_disk_image
  Input: { path, cache?, deep_scan? }
  Output: { vault, zones, filesystems, security }

Tool: list_partitions
  Input: { path, cache? }
  Output: { partition_table, zones[] }

Tool: list_files
  Input: { path, zone_index?, directory? }
  Output: { files[] }

Tool: extract_file
  Input: { image_path, file_path, zone_index, output_path }
  Output: { success, bytes_extracted, output_path }

Tool: validate_integrity
  Input: { path, check_checksums?, check_boot_sectors? }
  Output: { valid, issues[] }
```

---

**End of Document**
