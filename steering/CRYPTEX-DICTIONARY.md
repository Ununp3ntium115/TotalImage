# CRYPTEX-DICTIONARY: TotalImage Liberation Project

**Version:** 0.1.0-alpha
**Last Updated:** 2025-11-28
**Status:** Complete - All components documented

## Anarchist Terminology Framework

This cryptex-dictionary uses revolutionary terminology to document the complete architecture of TotalImage, mapping real Rust code names to branded anarchist terminology with full pseudocode references.

---

## Core Terminology

| Actual Name | Brand Name (Anarchist) | Concept | Pseudocode Ref |
|-------------|------------------------|---------|----------------|
| Component | **Cell** | Independent functional unit | All sections |
| Class/Module | **Collective** | Group of related operations | All sections |
| Method/Function | **Action** | Executable operation | All sections |
| Container Format | **Vault** | Encapsulated storage format | PSEUDOCODE 2.x |
| File System | **Territory** | Organized data domain | PSEUDOCODE 4.x |
| Partition | **Zone** | Segregated storage area | PSEUDOCODE 3.x |
| Read Operation | **Sabotage** | Extract data from proprietary formats | PSEUDOCODE 1.3 |
| Write Operation | **Propaganda** | Inject data into structures | PSEUDOCODE 10.x |
| Extract | **Liberation** | Free data from containers | PSEUDOCODE 1.3 |
| Parse | **Decrypt** | Decode structure | All parsers |
| Factory Pattern | **Underground Network** | Discovery and instantiation system | PSEUDOCODE 2.1 |
| Stream | **Pipeline** | Data flow channel | PSEUDOCODE 5.x |
| UI Layer | **Front** | Public-facing interface | PSEUDOCODE 7, 8 |
| Core Library | **Arsenal** | Core capabilities | PSEUDOCODE 1.x |
| Detection | **Reconnaissance** | Identify format/structure | PSEUDOCODE 2.1 |
| Boot Sector | **Manifesto** | System declaration | PSEUDOCODE 3.1, 4.1 |
| Dependencies | **Solidarity** | Inter-cell cooperation | All imports |
| Entry Point | **Ignition** | System activation | PSEUDOCODE 7.1 |
| Memory-mapped | **Direct Action** | Immediate access to resources | PSEUDOCODE 5.1 |

---

## Architecture Overview

**Project Codename:** TOTAL-LIBERATION
**Current State:** Rust + redb + Axum (Production Ready)
**Version:** 0.1.0-alpha

### Layer Structure

```
+---------------------------------------------+
|  FRONT (UI Layer)                           |  <- CLI + Web + MCP
|  CLI Front | Web Front | MCP Collective     |
+---------------------------------------------+
           | Solidarity (Dependencies) |
+---------------------------------------------+
|  ARSENAL (Core Library)                     |  <- Rust + redb
|  totalimage-core -> Core Traits & Types     |
+---------------------------------------------+
|  |-- Vault Cells (Container Handlers)       |  <- AFF4, E01, VHD, Raw
|  |-- Zone Cells (Partition Parsers)         |  <- MBR, GPT
|  |-- Territory Cells (Filesystem Impl)      |  <- FAT, NTFS, ISO, exFAT
|  |-- Pipeline Cells (I/O Abstractions)      |  <- Mmap, Partial
|  +-- Acquire Collective (Image Creation)    |  <- Raw, VHD creation
+---------------------------------------------+
           | Fire Marshal (Orchestration) |
+---------------------------------------------+
```

---

## Complete Cell Registry

### 1. Core Arsenal (totalimage-core)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `Vault` | Vault Interface | trait | 1.1 | Container format abstraction |
| `ZoneTable` | Zone Interface | trait | 1.2 | Partition table abstraction |
| `Territory` | Territory Interface | trait | 1.3 | Filesystem abstraction |
| `DirectoryCell` | Directory Interface | trait | 1.4 | Directory entry abstraction |
| `OccupantInfo` | Occupant Info | struct | 1.5 | File/directory metadata |
| `Zone` | Zone | struct | 1.6 | Partition metadata |
| `ReadSeek` | Read Pipeline | trait | 1.x | Read + Seek stream |
| `Error` | Liberation Error | enum | 1.7 | Error types |
| `validate_allocation_size` | Size Guard | function | 1.7 | Memory safety |
| `checked_multiply` | Overflow Guard | function | 1.7 | Integer safety |
| `validate_file_path` | Path Guard | function | 1.7 | Path traversal protection |

### 2. Vault Cells (totalimage-vaults)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `VaultType` | Vault Type | enum | 2.1 | Format identifier |
| `detect_vault_type` | Reconnaissance | function | 2.1 | Format detection |
| `open_vault` | Underground Network | function | 2.1 | Vault factory |
| `VaultConfig` | Vault Config | struct | 2.2 | Configuration options |
| `RawVault` | Raw Vault Cell | struct | 2.2 | DD/raw format handler |
| `VhdVault` | VHD Vault Cell | struct | 2.3 | VHD format handler |
| `VhdFooter` | VHD Manifesto | struct | 2.3 | VHD footer structure |
| `VhdDynamicHeader` | VHD Dynamic Header | struct | 2.3 | Dynamic disk header |
| `BlockAllocationTable` | BAT | struct | 2.3 | Block allocation table |
| `E01Vault` | E01 Vault Cell | struct | 2.4 | EnCase format handler |
| `E01FileHeader` | E01 Header | struct | 2.4 | E01 file header |
| `E01VolumeSection` | E01 Volume | struct | 2.4 | Volume metadata |
| `E01ChunkInfo` | E01 Chunk | struct | 2.4 | Chunk location info |
| `E01Cache` | E01 Cache | struct | 2.4 | Single-chunk cache |
| `Aff4Vault` | AFF4 Vault Cell | struct | 2.5 | AFF4 format handler |
| `Aff4Volume` | AFF4 Volume | struct | 2.5 | AFF4 volume metadata |
| `Aff4ImageStream` | AFF4 Stream | struct | 2.5 | Image stream metadata |
| `Aff4BevyIndexEntry` | AFF4 Bevy Index | struct | 2.5 | Chunk index entry |
| `TurtleParser` | Turtle Decryptor | struct | 2.5 | RDF turtle parser |
| `Aff4Compression` | AFF4 Compression | enum | 2.5 | Compression types |

### 3. Zone Cells (totalimage-zones)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `MbrZoneTable` | MBR Zone Table | struct | 3.1 | MBR parser |
| `MbrPartitionType` | MBR Partition Type | enum | 3.1 | MBR type codes |
| `MbrPartitionEntry` | MBR Entry | struct | 3.1 | Partition entry |
| `CHSAddress` | CHS Address | struct | 3.1 | Cylinder-Head-Sector |
| `GptZoneTable` | GPT Zone Table | struct | 3.2 | GPT parser |
| `GptHeader` | GPT Header | struct | 3.2 | GPT header structure |
| `GptPartitionEntry` | GPT Entry | struct | 3.2 | GPT partition entry |
| `PartitionTypeGuid` | Partition GUID | struct | 3.2 | Partition type GUID |

### 4. Territory Cells (totalimage-territories)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `FatTerritory` | FAT Territory | struct | 4.1 | FAT12/16/32 filesystem |
| `FatType` | FAT Type | enum | 4.1 | FAT variant |
| `BiosParameterBlock` | BPB Manifesto | struct | 4.1 | Boot sector BPB |
| `FatDirectoryEntry` | FAT Entry | struct | 4.1 | Directory entry |
| `FatDirectoryCell` | FAT Directory | struct | 4.1 | Directory implementation |
| `NtfsTerritory` | NTFS Territory | struct | 4.2 | NTFS filesystem |
| `NtfsVolumeInfo` | NTFS Volume Info | struct | 4.2 | Volume metadata |
| `NtfsDirectoryCell` | NTFS Directory | struct | 4.2 | Directory implementation |
| `IsoTerritory` | ISO Territory | struct | 4.3 | ISO 9660 filesystem |
| `PrimaryVolumeDescriptor` | ISO PVD | struct | 4.3 | Primary descriptor |
| `IsoDirectoryRecord` | ISO Record | struct | 4.3 | Directory record |
| `ExfatTerritory` | exFAT Territory | struct | 4.x | exFAT filesystem |

### 5. Pipeline Cells (totalimage-pipeline)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `MmapPipeline` | Direct Action Pipeline | struct | 5.1 | Memory-mapped I/O |
| `PartialPipeline` | Window Pipeline | struct | 5.2 | Windowed stream view |

### 6. MCP Collective (totalimage-mcp)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `MCPServer` | MCP Server | struct | 6.x | MCP protocol server |
| `MCPRequest` | MCP Request | enum | 6.1 | Request types |
| `MCPResponse` | MCP Response | struct | 6.1 | Response structure |
| `MCPError` | MCP Error | struct | 6.1 | Error response |
| `MCPErrorCode` | MCP Error Code | enum | 6.1 | JSON-RPC error codes |
| `Tool` | Tool Interface | trait | 6.2 | Tool abstraction |
| `ToolEnum` | Tool Enum | enum | 6.2 | Tool variants |
| `ToolDefinition` | Tool Definition | struct | 6.2 | Tool metadata |
| `ToolResult` | Tool Result | struct | 6.2 | Execution result |
| `AnalyzeDiskImageTool` | Analyze Tool | struct | 6.3 | Image analysis tool |
| `ListPartitionsTool` | List Partitions Tool | struct | 6.x | Partition listing |
| `ListFilesTool` | List Files Tool | struct | 6.x | File listing |
| `ExtractFileTool` | Extract Tool | struct | 6.x | File extraction |
| `ValidateIntegrityTool` | Validate Tool | struct | 6.x | Integrity checking |
| `ToolCache` | Tool Cache | struct | 6.4 | Result caching |
| `AuthConfig` | Auth Config | struct | 6.x | Authentication config |
| `WsState` | WebSocket State | struct | 6.x | WebSocket state |
| `WsMessage` | WebSocket Message | enum | 6.x | Progress messages |

### 7. CLI Front (totalimage-cli)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `main` | Ignition | function | 7.1 | Entry point |
| `cmd_info` | Info Action | function | 7.1 | Info command |
| `cmd_zones` | Zones Action | function | 7.1 | Zones command |
| `cmd_list` | List Action | function | 7.1 | List command |
| `cmd_extract` | Extract Action | function | 7.1 | Extract command |

### 8. Web Front (totalimage-web)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `/health` | Health Check | route | 8.1 | Health endpoint |
| `/api/vault/info` | Vault Info | route | 8.1 | Vault info endpoint |
| `/api/vault/zones` | Vault Zones | route | 8.1 | Zone list endpoint |
| `AppState` | App State | struct | 8.x | Server state |
| `MetadataCache` | Metadata Cache | struct | 8.x | Response cache |

### 9. Fire Marshal (fire-marshal)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `FireMarshal` | Fire Marshal | struct | 9.3 | Orchestration server |
| `FireMarshalConfig` | Marshal Config | struct | 9.3 | Server configuration |
| `ToolRegistry` | Tool Registry | struct | 9.1 | Tool registration |
| `ToolInfo` | Tool Info | struct | 9.1 | Tool metadata |
| `ToolExecutor` | Tool Executor | enum | 9.1 | Execution method |
| `RegisteredTool` | Registered Tool | struct | 9.1 | Registered tool info |
| `PlatformDatabase` | Platform Database | struct | 9.2 | Shared cache DB |
| `DatabaseConfig` | Database Config | struct | 9.2 | DB configuration |
| `CacheEntry` | Cache Entry | struct | 9.2 | Cached value wrapper |
| `DatabaseStats` | Database Stats | struct | 9.2 | Usage statistics |
| `HttpTransport` | HTTP Transport | struct | 9.x | HTTP tool calls |

### 10. Acquire Collective (totalimage-acquire)

| Real Name | Brand Name | Type | Pseudocode | Description |
|-----------|------------|------|------------|-------------|
| `RawAcquirer` | Raw Acquirer | struct | 10.1 | Raw image acquisition |
| `AcquireOptions` | Acquire Options | struct | 10.1 | Acquisition config |
| `AcquireProgress` | Acquire Progress | struct | 10.1 | Progress tracking |
| `ProgressCallback` | Progress Callback | trait | 10.1 | Progress interface |
| `VhdCreator` | VHD Creator | struct | 10.2 | VHD creation |
| `VhdOptions` | VHD Options | struct | 10.2 | VHD config |
| `VhdOutputType` | VHD Output Type | enum | 10.2 | Fixed/Dynamic |
| `VhdCreationResult` | VHD Result | struct | 10.2 | Creation result |
| `HashAlgorithm` | Hash Algorithm | enum | 10.x | MD5/SHA1/SHA256 |
| `Hasher` | Hasher | struct | 10.x | Hash computation |

---

## Security Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `MAX_SECTOR_SIZE` | 4,096 bytes | Sector size limit |
| `MAX_ALLOCATION_SIZE` | 256 MB | Memory allocation limit |
| `MAX_FAT_TABLE_SIZE` | 100 MB | FAT table size limit |
| `MAX_PARTITION_COUNT` | 256 | Partition count limit |
| `MAX_DIRECTORY_ENTRIES` | 10,000 | Directory listing limit |
| `MAX_FILE_EXTRACT_SIZE` | 1 GB | File extraction limit |
| `MAX_CLUSTER_CHAIN_LENGTH` | 1,000,000 | Chain traversal limit |
| `MAX_MMAP_SIZE` | 16 GB | Memory-map limit |

---

## File Signatures (Manifestos)

| Format | Signature | Offset | Brand Name |
|--------|-----------|--------|------------|
| VHD | `conectix` | EOF-512 | VHD Manifesto |
| E01 | `EVF\x09\x0D\x0A\xFF\x00` | 0 | E01 Manifesto |
| AFF4 | ZIP + `information.turtle` | 0 | AFF4 Manifesto |
| MBR | `0xAA55` | 510 | MBR Boot Signature |
| GPT | `EFI PART` | 512 | GPT Manifesto |
| FAT | `0xEB` or `0xE9` | 0 | FAT Jump Code |
| NTFS | `NTFS    ` | 3 | NTFS OEM ID |
| ISO | `CD001` | 32769 | ISO Identifier |

---

## Crate Dependencies (Solidarity Network)

```
totalimage-core (no internal deps)
    ^
    |
    +-- totalimage-pipeline
    |
    +-- totalimage-vaults
    |       |
    |       +-- uses: zip, flate2, md5, sha1, sha2
    |
    +-- totalimage-zones
    |       |
    |       +-- uses: crc32fast
    |
    +-- totalimage-territories
    |       |
    |       +-- uses: ntfs, encoding_rs
    |
    +-- totalimage-acquire
            |
            +-- uses: md5, sha1, sha2
    ^
    |
    +-- totalimage-cli
    |
    +-- totalimage-web
    |       |
    |       +-- uses: axum, tower, redb
    |
    +-- totalimage-mcp
            |
            +-- uses: axum, jsonwebtoken, redb, reqwest
            |
            v
        fire-marshal
            |
            +-- uses: axum, redb, governor
```

---

## Directory Structure

```
TotalImage/
|-- Cargo.toml                     # Workspace manifest
|-- crates/
|   |-- totalimage-core/           # Arsenal - Core traits
|   |-- totalimage-pipeline/       # Pipeline Cells - I/O
|   |-- totalimage-vaults/         # Vault Cells - Containers
|   |-- totalimage-zones/          # Zone Cells - Partitions
|   |-- totalimage-territories/    # Territory Cells - Filesystems
|   |-- totalimage-acquire/        # Acquire Collective - Creation
|   |-- totalimage-cli/            # CLI Front
|   |-- totalimage-web/            # Web Front
|   |-- totalimage-mcp/            # MCP Collective
|   +-- fire-marshal/              # Fire Marshal - Orchestration
|
|-- packages/
|   +-- pyro-worker-totalimage/    # PYRO Platform Worker
|
|-- steering/
|   |-- CRYPTEX-DICTIONARY.md      # This file
|   |-- PSEUDOCODE.md              # Full pseudocode specification
|   |-- SDLC-ROADMAP.md            # Development roadmap
|   |-- GAP-ANALYSIS.md            # Gap analysis
|   +-- STATUS-INDEX.md            # Status tracking
|
+-- release/
    +-- v0.1.0-alpha/              # Release binaries
```

---

## Document Cross-References

| Document | Purpose | Status |
|----------|---------|--------|
| `CRYPTEX-DICTIONARY.md` | Master terminology index | Complete |
| `PSEUDOCODE.md` | Full pseudocode specification | Complete |
| `SDLC-ROADMAP.md` | Development phases | Active |
| `GAP-ANALYSIS.md` | Known gaps and issues | Active |
| `STATUS-INDEX.md` | Current implementation status | Active |

---

## Conversion Status

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 1: Reconnaissance | Document all components | Complete |
| Phase 2: Arsenal Construction | Build Rust crates | Complete |
| Phase 3: Front Construction | CLI, Web, MCP interfaces | Complete |
| Phase 4: Liberation | Production deployment | Alpha Release |

---

**Document Status:** Complete
**Pseudocode Coverage:** 100%
**Next Update:** As codebase evolves
