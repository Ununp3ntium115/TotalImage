# MCP Server Module - Pseudocode Documentation

**Component:** `totalimage-mcp`  
**Location:** `crates/totalimage-mcp/src/`  
**Purpose:** Model Context Protocol server for Claude Desktop integration

---

## Table of Contents

1. [Overview](#overview)
2. [Server Architecture](#server-architecture)
3. [Tool Implementations](#tool-implementations)
4. [Protocol Handling](#protocol-handling)
5. [Code References](#code-references)

---

## Overview

The MCP server provides 5 tools for Claude Desktop:
- `analyze_disk_image`: Comprehensive analysis
- `list_partitions`: Zone enumeration
- `list_files`: Directory listing
- `extract_file`: File extraction
- `validate_integrity`: Forensic validation

**Code Reference:** `crates/totalimage-mcp/src/lib.rs:1-53`

---

## Server Architecture

### Dual-Mode Operation

**Code Reference:** `crates/totalimage-mcp/src/server.rs:29-115`

```pseudocode
ENUM ServerMode:
    Standalone(StandaloneConfig)    // stdio transport for Claude Desktop
    Integrated(IntegratedConfig)     // HTTP transport + Fire Marshal
END ENUM

STRUCTURE StandaloneConfig:
    cache_dir: PathBuf              // Cache directory
    config_file: OPTIONAL<PathBuf>  // Configuration file
END STRUCTURE

STRUCTURE IntegratedConfig:
    cache_dir: PathBuf              // Cache directory
    marshal_url: STRING              // Fire Marshal URL
    port: UINT16                     // HTTP server port
    tool_name: STRING                // Tool name for registry
    auth_config: OPTIONAL<AuthConfig> // Authentication
    websocket_enabled: BOOLEAN       // Enable WebSocket
END STRUCTURE

FUNCTION MCPServer.new_standalone(config: StandaloneConfig) -> Result<MCPServer>:
    // Create cache
    cache_path = config.cache_dir.join("mcp-cache.redb")
    cache = ToolCache::new(cache_path, "totalimage-mcp", VERSION)
    
    // Get allowed roots
    allowed_roots = resolve_allowed_roots()
    
    // Build tools
    tools = build_tools(cache, allowed_roots)
    
    RETURN MCPServer {
        mode: ServerMode::Standalone(config),
        tools: tools,
        cache: cache,
        allowed_roots: allowed_roots
    }
END FUNCTION

FUNCTION MCPServer.new_integrated(config: IntegratedConfig) -> Result<MCPServer>:
    // Similar to standalone but with HTTP transport
    cache_path = config.cache_dir.join("mcp-cache.redb")
    cache = ToolCache::new(cache_path, config.tool_name, VERSION)
    
    allowed_roots = resolve_allowed_roots()
    tools = build_tools(cache, allowed_roots)
    
    RETURN MCPServer {
        mode: ServerMode::Integrated(config),
        tools: tools,
        cache: cache,
        allowed_roots: allowed_roots
    }
END FUNCTION
```

---

## Tool Implementations

### Analyze Disk Image Tool

**Code Reference:** `crates/totalimage-mcp/src/tools.rs:145-300`

```pseudocode
STRUCTURE AnalyzeDiskImageTool:
    cache: Arc<ToolCache>
    allowed_roots: Arc<ARRAY<PathBuf>>
END STRUCTURE

FUNCTION AnalyzeDiskImageTool.execute(args: OPTIONAL<Value>) -> Result<ToolResult>:
    // Parse arguments
    input = parse_args<AnalyzeDiskImageInput>(args)
    
    // Validate path
    path = validate_file_path(input.path, this.allowed_roots)
    
    // Check cache
    IF input.cache != false:
        cached = this.cache.get(input.path)
        IF cached IS NOT NULL:
            RETURN ToolResult::success(cached)
    END IF
    
    // Open vault
    vault = open_vault(path, VaultConfig::default())
    
    // Parse zones
    zones = []
    vault_content = vault.content()
    zone_table = detect_zone_table(vault_content)
    IF zone_table IS NOT NULL:
        zones = zone_table.enumerate_zones()
    END IF
    
    // Analyze filesystems
    filesystems = []
    FOR EACH zone IN zones:
        territory = detect_territory(vault_content, zone)
        IF territory IS NOT NULL:
            filesystems.append({
                zone_index: zone.index,
                type: territory.identify(),
                label: territory.banner(),
                total_size: territory.domain_size(),
                free_size: territory.liberated_space()
            })
        END IF
    END FOR
    
    // Security validation
    security = validate_security(vault, zones)
    
    // Build result
    result = {
        vault: {
            type: vault.identify(),
            size: vault.length()
        },
        zones: zones.map(zone => {
            index: zone.index,
            type: zone.zone_type,
            offset: zone.offset,
            length: zone.length
        }),
        filesystems: filesystems,
        security: security
    }
    
    // Cache result
    IF input.cache != false:
        this.cache.set(input.path, result)
    END IF
    
    RETURN ToolResult::success(result)
END FUNCTION
```

### List Partitions Tool

**Code Reference:** `crates/totalimage-mcp/src/tools.rs:302-400`

```pseudocode
FUNCTION ListPartitionsTool.execute(args: OPTIONAL<Value>) -> Result<ToolResult>:
    input = parse_args<ListPartitionsInput>(args)
    path = validate_file_path(input.path, this.allowed_roots)
    
    // Check cache
    IF input.cache != false:
        cached = this.cache.get(format("partitions:{}", input.path))
        IF cached IS NOT NULL:
            RETURN ToolResult::success(cached)
    END IF
    
    // Open vault and parse zones
    vault = open_vault(path, VaultConfig::default())
    vault_content = vault.content()
    zone_table = detect_zone_table(vault_content)
    
    IF zone_table IS NULL:
        RETURN ToolResult::success({
            partition_table: "None",
            zones: []
        })
    END IF
    
    zones = zone_table.enumerate_zones()
    
    result = {
        partition_table: zone_table.identify(),
        zones: zones.map(zone => {
            index: zone.index,
            type: zone.zone_type,
            offset: zone.offset,
            length: zone.length
        })
    }
    
    // Cache result
    IF input.cache != false:
        this.cache.set(format("partitions:{}", input.path), result)
    END IF
    
    RETURN ToolResult::success(result)
END FUNCTION
```

### List Files Tool

**Code Reference:** `crates/totalimage-mcp/src/tools.rs:402-550`

```pseudocode
FUNCTION ListFilesTool.execute(args: OPTIONAL<Value>) -> Result<ToolResult>:
    input = parse_args<ListFilesInput>(args)
    path = validate_file_path(input.image_path, this.allowed_roots)
    
    // Open vault
    vault = open_vault(path, VaultConfig::default())
    vault_content = vault.content()
    
    // Get zone
    zone_table = detect_zone_table(vault_content)
    zone = zone_table.get_zone(input.zone_index OR 0)
    IF zone IS NULL:
        RETURN ToolResult::error("Zone not found")
    END IF
    
    // Parse territory
    zone_stream = PartialPipeline::new(vault_content, zone.offset, zone.length)
    territory = detect_and_parse_territory(zone_stream)
    IF territory IS NULL:
        RETURN ToolResult::error("No filesystem found in zone")
    END IF
    
    // Navigate to directory
    directory = IF input.directory IS NOT NULL:
        territory.navigate_to(input.directory)
    ELSE:
        territory.headquarters()
    END IF
    
    // List occupants
    occupants = directory.list_occupants()
    
    result = {
        files: occupants.map(occupant => {
            name: occupant.name,
            size: occupant.size,
            is_directory: occupant.is_directory,
            created: occupant.created,
            modified: occupant.modified
        })
    }
    
    RETURN ToolResult::success(result)
END FUNCTION
```

### Extract File Tool

**Code Reference:** `crates/totalimage-mcp/src/tools.rs:552-700`

```pseudocode
FUNCTION ExtractFileTool.execute(args: OPTIONAL<Value>) -> Result<ToolResult>:
    input = parse_args<ExtractFileInput>(args)
    image_path = validate_file_path(input.image_path, this.allowed_roots)
    output_path = validate_file_path(input.output_path, this.allowed_roots)
    
    // Open vault
    vault = open_vault(image_path, VaultConfig::default())
    vault_content = vault.content()
    
    // Get zone and territory
    zone_table = detect_zone_table(vault_content)
    zone = zone_table.get_zone(input.zone_index)
    zone_stream = PartialPipeline::new(vault_content, zone.offset, zone.length)
    territory = detect_and_parse_territory(zone_stream)
    
    // Extract file
    file_data = territory.extract_file(input.file_path)
    
    // Write to output
    write_file(output_path, file_data)
    
    result = {
        success: true,
        bytes_extracted: file_data.length,
        output_path: output_path
    }
    
    RETURN ToolResult::success(result)
END FUNCTION
```

### Validate Integrity Tool

**Code Reference:** `crates/totalimage-mcp/src/tools.rs:702-850`

```pseudocode
FUNCTION ValidateIntegrityTool.execute(args: OPTIONAL<Value>) -> Result<ToolResult>:
    input = parse_args<ValidateIntegrityInput>(args)
    path = validate_file_path(input.path, this.allowed_roots)
    
    issues = []
    
    // Open vault
    vault = open_vault(path, VaultConfig::default())
    
    // Validate vault checksums
    IF input.check_checksums != false:
        IF vault IS VhdVault:
            IF NOT vault.footer.verify_checksum():
                issues.append("VHD footer checksum invalid")
        ELSE IF vault IS E01Vault:
            IF vault.hash IS NOT NULL:
                calculated = calculate_md5(vault)
                IF calculated != vault.hash.md5_hash:
                    issues.append("E01 MD5 hash mismatch")
    END IF
    
    // Validate boot sectors
    IF input.check_boot_sectors != false:
        vault_content = vault.content()
        zone_table = detect_zone_table(vault_content)
        
        FOR EACH zone IN zone_table.enumerate_zones():
            zone_stream = PartialPipeline::new(vault_content, zone.offset, zone.length)
            territory = detect_and_parse_territory(zone_stream)
            
            IF territory IS FatTerritory:
                // Validate FAT boot sector signature
                IF NOT validate_fat_boot_sector(zone_stream):
                    issues.append(format("Zone {}: Invalid FAT boot sector", zone.index))
        END FOR
    END IF
    
    result = {
        valid: issues IS EMPTY,
        issues: issues
    }
    
    RETURN ToolResult::success(result)
END FUNCTION
```

---

## Protocol Handling

### MCP Request Processing

**Code Reference:** `crates/totalimage-mcp/src/protocol.rs`

```pseudocode
FUNCTION handle_mcp_request(request: MCPRequest) -> MCPResponse:
    SWITCH request.method:
        CASE "initialize":
            RETURN handle_initialize(request.params)
        
        CASE "tools/list":
            RETURN handle_list_tools()
        
        CASE "tools/call":
            RETURN handle_call_tool(request.params)
    END SWITCH
END FUNCTION

FUNCTION handle_call_tool(params: CallToolParams) -> MCPResponse:
    // Find tool
    tool = find_tool(params.name)
    IF tool IS NULL:
        RETURN MCPResponse::error("Tool not found")
    END IF
    
    // Execute tool
    result = tool.execute(params.arguments)
    
    IF result.success:
        RETURN MCPResponse::success(result.data)
    ELSE:
        RETURN MCPResponse::error(result.error)
    END IF
END FUNCTION
```

---

## Code References

### File Structure

```
crates/totalimage-mcp/src/
├── lib.rs              # Module exports (lines 1-53)
├── main.rs             # Binary entry point (lines 1-187)
├── server.rs           # Server implementation (lines 1-500+)
├── tools.rs            # Tool implementations (lines 1-1371)
├── protocol.rs         # MCP protocol (lines 1-300+)
├── cache.rs            # Tool cache
├── auth.rs             # Authentication
├── metrics.rs          # Metrics collection
└── websocket.rs        # WebSocket support
```

### Key Functions

#### `tools.rs`
- `AnalyzeDiskImageTool::execute`: `crates/totalimage-mcp/src/tools.rs:150-300`
- `ListPartitionsTool::execute`: `crates/totalimage-mcp/src/tools.rs:302-400`
- `ListFilesTool::execute`: `crates/totalimage-mcp/src/tools.rs:402-550`
- `ExtractFileTool::execute`: `crates/totalimage-mcp/src/tools.rs:552-700`
- `ValidateIntegrityTool::execute`: `crates/totalimage-mcp/src/tools.rs:702-850`

#### `server.rs`
- `MCPServer::new_standalone`: `crates/totalimage-mcp/src/server.rs:68-85`
- `MCPServer::new_integrated`: `crates/totalimage-mcp/src/server.rs:88-105`
- `MCPServer::listen_stdio`: stdio transport
- `MCPServer::listen_http`: HTTP transport

---

## Cross-References

- **Core Usage:** See [01-core.md](01-core.md) (uses Vault, Territory, ZoneTable)
- **Vault Usage:** See [02-vaults.md](02-vaults.md) (opens vaults)
- **Zone Usage:** See [03-zones.md](03-zones.md) (parses zones)
- **Territory Usage:** See [04-territories.md](04-territories.md) (parses filesystems)
- **Fire Marshal Integration:** See [08-fire-marshal.md](08-fire-marshal.md)

---

**Document Version:** 1.0.0  
**Last Updated:** December 21, 2025  
**Next:** [08-fire-marshal.md](08-fire-marshal.md) - Fire Marshal API
