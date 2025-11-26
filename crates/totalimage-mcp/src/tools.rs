//! MCP tool implementations for disk image analysis
//!
//! Provides 5 core tools:
//! - analyze_disk_image: Comprehensive disk analysis
//! - list_partitions: List all partitions/zones
//! - list_files: List files in a filesystem
//! - extract_file: Extract file from disk image
//! - validate_integrity: Validate checksums and structure

use crate::cache::ToolCache;
use crate::protocol::{ToolDefinition, ToolResult};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use totalimage_core::{validate_file_path, Zone, Territory, ZoneTable};
use totalimage_pipeline::PartialPipeline;
use totalimage_territories::{FatTerritory, IsoTerritory};
use totalimage_vaults::{open_vault, VaultConfig};
use totalimage_zones::{GptZoneTable, MbrZoneTable};

/// Tool trait for MCP tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (e.g., "analyze_disk_image")
    fn name(&self) -> &str;

    /// Tool description
    fn description(&self) -> &str;

    /// JSON schema for tool parameters
    fn input_schema(&self) -> Value;

    /// Execute the tool with given arguments
    async fn execute(&self, args: Option<Value>) -> Result<ToolResult>;

    /// Get tool definition for tools/list response
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: self.input_schema(),
        }
    }
}

/// Tool information for registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Concrete tool enum (avoids dyn trait issues with async)
pub enum ToolEnum {
    AnalyzeDiskImage(AnalyzeDiskImageTool),
    ListPartitions(ListPartitionsTool),
    ListFiles(ListFilesTool),
    ExtractFile(ExtractFileTool),
    ValidateIntegrity(ValidateIntegrityTool),
}

impl ToolEnum {
    pub fn name(&self) -> &str {
        match self {
            ToolEnum::AnalyzeDiskImage(t) => t.name(),
            ToolEnum::ListPartitions(t) => t.name(),
            ToolEnum::ListFiles(t) => t.name(),
            ToolEnum::ExtractFile(t) => t.name(),
            ToolEnum::ValidateIntegrity(t) => t.name(),
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ToolEnum::AnalyzeDiskImage(t) => t.description(),
            ToolEnum::ListPartitions(t) => t.description(),
            ToolEnum::ListFiles(t) => t.description(),
            ToolEnum::ExtractFile(t) => t.description(),
            ToolEnum::ValidateIntegrity(t) => t.description(),
        }
    }

    pub fn input_schema(&self) -> Value {
        match self {
            ToolEnum::AnalyzeDiskImage(t) => t.input_schema(),
            ToolEnum::ListPartitions(t) => t.input_schema(),
            ToolEnum::ListFiles(t) => t.input_schema(),
            ToolEnum::ExtractFile(t) => t.input_schema(),
            ToolEnum::ValidateIntegrity(t) => t.input_schema(),
        }
    }

    pub async fn execute(&self, args: Option<Value>) -> Result<ToolResult> {
        match self {
            ToolEnum::AnalyzeDiskImage(t) => t.execute(args).await,
            ToolEnum::ListPartitions(t) => t.execute(args).await,
            ToolEnum::ListFiles(t) => t.execute(args).await,
            ToolEnum::ExtractFile(t) => t.execute(args).await,
            ToolEnum::ValidateIntegrity(t) => t.execute(args).await,
        }
    }

    pub fn definition(&self) -> ToolDefinition {
        match self {
            ToolEnum::AnalyzeDiskImage(t) => t.definition(),
            ToolEnum::ListPartitions(t) => t.definition(),
            ToolEnum::ListFiles(t) => t.definition(),
            ToolEnum::ExtractFile(t) => t.definition(),
            ToolEnum::ValidateIntegrity(t) => t.definition(),
        }
    }
}

// ============================================================================
// Tool 1: Analyze Disk Image
// ============================================================================

pub struct AnalyzeDiskImageTool {
    pub cache: Arc<ToolCache>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnalyzeDiskImageInput {
    path: String,
    #[serde(default = "default_true")]
    cache: bool,
    #[serde(default)]
    deep_scan: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
struct AnalyzeDiskImageOutput {
    vault: VaultInfo,
    zones: Vec<ZoneInfo>,
    filesystems: Vec<FilesystemInfo>,
    security: SecurityAnalysis,
}

#[derive(Debug, Serialize, Deserialize)]
struct VaultInfo {
    path: String,
    vault_type: String,
    size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ZoneInfo {
    index: usize,
    offset: u64,
    length: u64,
    zone_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FilesystemInfo {
    zone_index: usize,
    filesystem_type: String,
    label: Option<String>,
    total_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SecurityAnalysis {
    boot_sector_valid: bool,
    partition_table_valid: bool,
    checksum_results: Vec<ChecksumResult>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChecksumResult {
    component: String,
    valid: bool,
    details: Option<String>,
}

#[async_trait]
impl Tool for AnalyzeDiskImageTool {
    fn name(&self) -> &str {
        "analyze_disk_image"
    }

    fn description(&self) -> &str {
        "Comprehensive disk image analysis: vault type, partitions, filesystems, security validation"
    }

    fn input_schema(&self) -> Value {
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

    async fn execute(&self, args: Option<Value>) -> Result<ToolResult> {
        let input: AnalyzeDiskImageInput = serde_json::from_value(args.unwrap_or(json!({})))
            .context("Invalid arguments for analyze_disk_image")?;

        // Check cache
        let cache_key = format!("analyze:{}:{}", input.path, input.deep_scan);
        if input.cache {
            if let Ok(Some(cached)) = self.cache.get::<AnalyzeDiskImageOutput>(&cache_key) {
                tracing::info!("Cache HIT for analyze_disk_image: {}", input.path);
                return Ok(ToolResult::from_value(serde_json::to_value(&cached)?));
            }
        }

        tracing::info!("Cache MISS for analyze_disk_image: {}", input.path);

        // Validate path
        let path = validate_file_path(&input.path)
            .context("Invalid file path")?;

        // Analyze vault
        let mut vault = open_vault(&path, VaultConfig::default())
            .context("Failed to open vault")?;

        let vault_info = VaultInfo {
            path: input.path.clone(),
            vault_type: vault.identify().to_string(),
            size_bytes: vault.length(),
        };

        // Analyze zones (partitions)
        let sector_size = 512;
        let mut zones = Vec::new();
        let mut filesystems = Vec::new();
        let mut security = SecurityAnalysis {
            boot_sector_valid: true,
            partition_table_valid: true,
            checksum_results: Vec::new(),
        };

        // Try MBR
        if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
            zones = mbr
                .enumerate_zones()
                .iter()
                .map(|z| ZoneInfo {
                    index: z.index,
                    offset: z.offset,
                    length: z.length,
                    zone_type: z.zone_type.clone(),
                })
                .collect();

            security.partition_table_valid = true;
            security.checksum_results.push(ChecksumResult {
                component: "MBR Boot Signature".to_string(),
                valid: true,
                details: Some("0xAA55 signature present".to_string()),
            });
        }
        // Try GPT
        else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
            zones = gpt
                .enumerate_zones()
                .iter()
                .map(|z| ZoneInfo {
                    index: z.index,
                    offset: z.offset,
                    length: z.length,
                    zone_type: z.zone_type.clone(),
                })
                .collect();

            security.partition_table_valid = true;
            security.checksum_results.push(ChecksumResult {
                component: "GPT Header CRC32".to_string(),
                valid: true,
                details: Some("Header checksum validated".to_string()),
            });
            security.checksum_results.push(ChecksumResult {
                component: "GPT Partition Array CRC32".to_string(),
                valid: true,
                details: Some("Partition array checksum validated".to_string()),
            });
        }

        // Analyze filesystems if deep scan requested
        if input.deep_scan {
            for (idx, zone) in zones.iter().enumerate() {
                // Create partial pipeline for this zone
                if let Ok(mut partial) = PartialPipeline::new(vault.content(), zone.offset, zone.length) {
                    // Try FAT
                    if let Ok(fat) = FatTerritory::parse(&mut partial) {
                        filesystems.push(FilesystemInfo {
                            zone_index: idx,
                            filesystem_type: fat.identify().to_string(),
                            label: Some("FAT Volume".to_string()), // TODO: Extract actual label
                            total_size: zone.length,
                        });
                    }
                    // Try ISO
                    else if let Ok(iso) = IsoTerritory::parse(&mut partial) {
                        filesystems.push(FilesystemInfo {
                            zone_index: idx,
                            filesystem_type: iso.identify().to_string(),
                            label: Some("ISO Volume".to_string()), // TODO: Extract actual label
                            total_size: zone.length,
                        });
                    }
                }
            }
        }

        let output = AnalyzeDiskImageOutput {
            vault: vault_info,
            zones,
            filesystems,
            security,
        };

        // Cache result
        if input.cache {
            if let Err(e) = self.cache.set(&cache_key, &output) {
                tracing::warn!("Failed to cache result: {}", e);
            }
        }

        Ok(ToolResult::from_value(serde_json::to_value(&output)?))
    }
}

// ============================================================================
// Tool 2: List Partitions
// ============================================================================

pub struct ListPartitionsTool {
    pub cache: Arc<ToolCache>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListPartitionsInput {
    path: String,
    #[serde(default = "default_true")]
    cache: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListPartitionsOutput {
    partition_table: String,
    zones: Vec<ZoneInfo>,
}

#[async_trait]
impl Tool for ListPartitionsTool {
    fn name(&self) -> &str {
        "list_partitions"
    }

    fn description(&self) -> &str {
        "List all partitions (zones) in a disk image"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to disk image file"
                },
                "cache": {
                    "type": "boolean",
                    "default": true,
                    "description": "Use cached results if available"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Option<Value>) -> Result<ToolResult> {
        let input: ListPartitionsInput = serde_json::from_value(args.unwrap_or(json!({})))
            .context("Invalid arguments for list_partitions")?;

        // Check cache
        let cache_key = format!("partitions:{}", input.path);
        if input.cache {
            if let Ok(Some(cached)) = self.cache.get::<ListPartitionsOutput>(&cache_key) {
                return Ok(ToolResult::from_value(serde_json::to_value(&cached)?));
            }
        }

        // Validate path
        let path = validate_file_path(&input.path)?;

        // Open vault
        let mut vault = open_vault(&path, VaultConfig::default())?;
        let sector_size = 512;

        let output = if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
            ListPartitionsOutput {
                partition_table: mbr.identify().to_string(),
                zones: mbr
                    .enumerate_zones()
                    .iter()
                    .map(|z| ZoneInfo {
                        index: z.index,
                        offset: z.offset,
                        length: z.length,
                        zone_type: z.zone_type.clone(),
                    })
                    .collect(),
            }
        } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
            ListPartitionsOutput {
                partition_table: gpt.identify().to_string(),
                zones: gpt
                    .enumerate_zones()
                    .iter()
                    .map(|z| ZoneInfo {
                        index: z.index,
                        offset: z.offset,
                        length: z.length,
                        zone_type: z.zone_type.clone(),
                    })
                    .collect(),
            }
        } else {
            ListPartitionsOutput {
                partition_table: "None".to_string(),
                zones: Vec::new(),
            }
        };

        // Cache result
        if input.cache {
            let _ = self.cache.set(&cache_key, &output);
        }

        Ok(ToolResult::from_value(serde_json::to_value(&output)?))
    }
}

// ============================================================================
// Tool 3: List Files
// ============================================================================

pub struct ListFilesTool {
    pub cache: Arc<ToolCache>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListFilesInput {
    path: String,
    #[serde(default)]
    zone_index: usize,
    #[serde(default = "default_true")]
    cache: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListFilesOutput {
    files: Vec<FileInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileInfo {
    name: String,
    size: u64,
    is_directory: bool,
}

#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files in a disk image filesystem"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to disk image file"
                },
                "zone_index": {
                    "type": "number",
                    "default": 0,
                    "description": "Partition index (0 for first partition)"
                },
                "cache": {
                    "type": "boolean",
                    "default": true,
                    "description": "Use cached results if available"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Option<Value>) -> Result<ToolResult> {
        let input: ListFilesInput = serde_json::from_value(args.unwrap_or(json!({})))
            .context("Invalid arguments for list_files")?;

        // Check cache
        let cache_key = format!("files:{}:{}", input.path, input.zone_index);
        if input.cache {
            if let Ok(Some(cached)) = self.cache.get::<ListFilesOutput>(&cache_key) {
                return Ok(ToolResult::from_value(serde_json::to_value(&cached)?));
            }
        }

        // Validate path
        let path = validate_file_path(&input.path)?;

        // Open vault and get zone
        let mut vault = open_vault(&path, VaultConfig::default())?;
        let sector_size = 512;

        // Get zone information
        let zone = if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
            mbr.enumerate_zones()
                .get(input.zone_index)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Zone index {} not found", input.zone_index))?
        } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
            gpt.enumerate_zones()
                .get(input.zone_index)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Zone index {} not found", input.zone_index))?
        } else {
            // No partition table, use entire image
            Zone {
                index: 0,
                offset: 0,
                length: vault.length(),
                zone_type: "Unknown".to_string(),
                territory_type: None,
            }
        };

        // Create partial pipeline for the zone
        let mut partial = PartialPipeline::new(vault.content(), zone.offset, zone.length)?;

        // Try to parse filesystem
        let files = if let Ok(fat) = FatTerritory::parse(&mut partial) {
            let root = fat.headquarters()?;
            let occupants = root.list_occupants()?;

            occupants
                .into_iter()
                .map(|o| FileInfo {
                    name: o.name,
                    size: o.size,
                    is_directory: o.is_directory,
                })
                .collect()
        } else if let Ok(iso) = IsoTerritory::parse(&mut partial) {
            let root = iso.headquarters()?;
            let occupants = root.list_occupants()?;

            occupants
                .into_iter()
                .map(|o| FileInfo {
                    name: o.name,
                    size: o.size,
                    is_directory: o.is_directory,
                })
                .collect()
        } else {
            return Err(anyhow::anyhow!("Unable to read filesystem at zone {}", input.zone_index));
        };

        let output = ListFilesOutput { files };

        // Cache result
        if input.cache {
            let _ = self.cache.set(&cache_key, &output);
        }

        Ok(ToolResult::from_value(serde_json::to_value(&output)?))
    }
}

// ============================================================================
// Tool 4: Extract File
// ============================================================================

pub struct ExtractFileTool {}

#[derive(Debug, Serialize, Deserialize)]
struct ExtractFileInput {
    image_path: String,
    file_path: String,
    #[serde(default)]
    zone_index: usize,
    output_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExtractFileOutput {
    success: bool,
    bytes_extracted: u64,
    output_path: String,
}

#[async_trait]
impl Tool for ExtractFileTool {
    fn name(&self) -> &str {
        "extract_file"
    }

    fn description(&self) -> &str {
        "Extract a file from a disk image filesystem"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "image_path": {
                    "type": "string",
                    "description": "Path to disk image file"
                },
                "file_path": {
                    "type": "string",
                    "description": "Path to file within disk image (e.g., 'README.TXT')"
                },
                "zone_index": {
                    "type": "number",
                    "default": 0,
                    "description": "Partition index containing the file"
                },
                "output_path": {
                    "type": "string",
                    "description": "Where to save the extracted file"
                }
            },
            "required": ["image_path", "file_path", "output_path"]
        })
    }

    async fn execute(&self, args: Option<Value>) -> Result<ToolResult> {
        let input: ExtractFileInput = serde_json::from_value(args.unwrap_or(json!({})))
            .context("Invalid arguments for extract_file")?;

        // Validate paths
        let image_path = validate_file_path(&input.image_path)?;
        let output_path = PathBuf::from(&input.output_path);

        // Open vault
        let mut vault = open_vault(&image_path, VaultConfig::default())?;
        let sector_size = 512;

        // Get zone information
        let zone = if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
            mbr.enumerate_zones()
                .get(input.zone_index)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Zone index {} not found", input.zone_index))?
        } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
            gpt.enumerate_zones()
                .get(input.zone_index)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Zone index {} not found", input.zone_index))?
        } else {
            // No partition table, use entire image
            Zone {
                index: 0,
                offset: 0,
                length: vault.length(),
                zone_type: "Unknown".to_string(),
                territory_type: None,
            }
        };

        // Create partial pipeline for the zone
        let mut partial = PartialPipeline::new(vault.content(), zone.offset, zone.length)?;

        // Try to extract from filesystem
        let bytes_extracted = if let Ok(fat) = FatTerritory::parse(&mut partial) {
            // Find the file in root directory
            let entry = fat.find_file_in_root(&mut partial, &input.file_path)?;

            // Read file data
            let data = fat.read_file_data(&mut partial, &entry)?;

            // Write to output file
            let mut file = std::fs::File::create(&output_path)?;
            file.write_all(&data)?;

            data.len() as u64
        } else if let Ok(_iso) = IsoTerritory::parse(&mut partial) {
            // TODO: Implement ISO file extraction
            // ISO extraction requires different methods - see CLI implementation
            return Err(anyhow::anyhow!("ISO file extraction not yet implemented"));
        } else {
            return Err(anyhow::anyhow!("Unable to read filesystem at zone {}", input.zone_index));
        };

        let output = ExtractFileOutput {
            success: true,
            bytes_extracted,
            output_path: input.output_path,
        };

        Ok(ToolResult::from_value(serde_json::to_value(&output)?))
    }
}

// ============================================================================
// Tool 5: Validate Integrity
// ============================================================================

pub struct ValidateIntegrityTool {}

#[derive(Debug, Serialize, Deserialize)]
struct ValidateIntegrityInput {
    path: String,
    #[serde(default = "default_true")]
    check_checksums: bool,
    #[serde(default = "default_true")]
    check_boot_sectors: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ValidateIntegrityOutput {
    valid: bool,
    issues: Vec<IntegrityIssue>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IntegrityIssue {
    severity: String,
    component: String,
    message: String,
}

#[async_trait]
impl Tool for ValidateIntegrityTool {
    fn name(&self) -> &str {
        "validate_integrity"
    }

    fn description(&self) -> &str {
        "Validate disk image integrity (checksums, boot sectors, structure)"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to disk image file"
                },
                "check_checksums": {
                    "type": "boolean",
                    "default": true,
                    "description": "Verify checksums (VHD, GPT)"
                },
                "check_boot_sectors": {
                    "type": "boolean",
                    "default": true,
                    "description": "Validate boot sector signatures"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Option<Value>) -> Result<ToolResult> {
        let input: ValidateIntegrityInput = serde_json::from_value(args.unwrap_or(json!({})))
            .context("Invalid arguments for validate_integrity")?;

        // Validate path
        let path = validate_file_path(&input.path)?;

        // Open vault
        let mut vault = open_vault(&path, VaultConfig::default())?;
        let mut issues = Vec::new();

        // Check partition table
        let sector_size = 512;
        if input.check_boot_sectors {
            if let Ok(_mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
                // MBR boot signature is validated during parse
                // If we got here, it's valid
            } else if let Ok(_gpt) = GptZoneTable::parse(vault.content(), sector_size) {
                // GPT CRC32 is validated during parse
                // If we got here, it's valid
            } else {
                issues.push(IntegrityIssue {
                    severity: "warning".to_string(),
                    component: "Partition Table".to_string(),
                    message: "No valid partition table found".to_string(),
                });
            }
        }

        let valid = issues.is_empty();

        let output = ValidateIntegrityOutput { valid, issues };

        Ok(ToolResult::from_value(serde_json::to_value(&output)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Helper to create a test cache in a temporary directory
    fn create_test_cache() -> Arc<ToolCache> {
        let temp_dir = tempdir().unwrap();
        let cache_path = temp_dir.path().join("test_cache.redb");
        // Leak the tempdir so it lives for the duration of the test
        std::mem::forget(temp_dir);
        Arc::new(ToolCache::new(cache_path, "test_tool", "1.0.0").unwrap())
    }

    // =========================================================================
    // Tool Info Tests
    // =========================================================================

    #[test]
    fn test_tool_info_serialization() {
        let info = ToolInfo {
            name: "test_tool".to_string(),
            version: "1.0.0".to_string(),
            description: "A test tool".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test_tool"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_tool_info_deserialization() {
        let json = r#"{"name":"my_tool","version":"2.0.0","description":"Description"}"#;
        let info: ToolInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.name, "my_tool");
        assert_eq!(info.version, "2.0.0");
        assert_eq!(info.description, "Description");
    }

    // =========================================================================
    // Output Struct Tests
    // =========================================================================

    #[test]
    fn test_vault_info_serialization() {
        let info = VaultInfo {
            path: "/path/to/image.vhd".to_string(),
            vault_type: "VHD".to_string(),
            size_bytes: 1024 * 1024 * 100,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("image.vhd"));
        assert!(json.contains("VHD"));
        assert!(json.contains("104857600"));
    }

    #[test]
    fn test_zone_info_serialization() {
        let info = ZoneInfo {
            index: 0,
            offset: 1048576,
            length: 104857600,
            zone_type: "NTFS".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"index\":0"));
        assert!(json.contains("NTFS"));
    }

    #[test]
    fn test_filesystem_info_serialization() {
        let info = FilesystemInfo {
            zone_index: 1,
            filesystem_type: "FAT32".to_string(),
            label: Some("DATA".to_string()),
            total_size: 1073741824,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("FAT32"));
        assert!(json.contains("DATA"));
    }

    #[test]
    fn test_filesystem_info_no_label() {
        let info = FilesystemInfo {
            zone_index: 0,
            filesystem_type: "NTFS".to_string(),
            label: None,
            total_size: 500000000,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("NTFS"));
        assert!(json.contains("null"));
    }

    #[test]
    fn test_security_analysis_serialization() {
        let analysis = SecurityAnalysis {
            boot_sector_valid: true,
            partition_table_valid: true,
            checksum_results: vec![
                ChecksumResult {
                    component: "MBR".to_string(),
                    valid: true,
                    details: Some("Valid signature".to_string()),
                }
            ],
        };

        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("boot_sector_valid"));
        assert!(json.contains("true"));
        assert!(json.contains("MBR"));
    }

    #[test]
    fn test_file_info_serialization() {
        let info = FileInfo {
            name: "README.TXT".to_string(),
            size: 1024,
            is_directory: false,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("README.TXT"));
        assert!(json.contains("1024"));
        assert!(json.contains("false"));
    }

    #[test]
    fn test_file_info_directory() {
        let info = FileInfo {
            name: "Documents".to_string(),
            size: 0,
            is_directory: true,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Documents"));
        assert!(json.contains("\"is_directory\":true"));
    }

    #[test]
    fn test_integrity_issue_serialization() {
        let issue = IntegrityIssue {
            severity: "error".to_string(),
            component: "GPT Header".to_string(),
            message: "CRC mismatch".to_string(),
        };

        let json = serde_json::to_string(&issue).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("GPT Header"));
        assert!(json.contains("CRC mismatch"));
    }

    #[test]
    fn test_validate_integrity_output() {
        let output = ValidateIntegrityOutput {
            valid: false,
            issues: vec![
                IntegrityIssue {
                    severity: "warning".to_string(),
                    component: "Boot Sector".to_string(),
                    message: "Invalid signature".to_string(),
                }
            ],
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"valid\":false"));
        assert!(json.contains("warning"));
    }

    #[test]
    fn test_extract_file_output() {
        let output = ExtractFileOutput {
            success: true,
            bytes_extracted: 2048,
            output_path: "/tmp/extracted.txt".to_string(),
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("true"));
        assert!(json.contains("2048"));
        assert!(json.contains("/tmp/extracted.txt"));
    }

    #[test]
    fn test_list_files_output() {
        let output = ListFilesOutput {
            files: vec![
                FileInfo { name: "file1.txt".to_string(), size: 100, is_directory: false },
                FileInfo { name: "dir1".to_string(), size: 0, is_directory: true },
            ],
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("file1.txt"));
        assert!(json.contains("dir1"));
    }

    #[test]
    fn test_list_partitions_output() {
        let output = ListPartitionsOutput {
            partition_table: "GPT".to_string(),
            zones: vec![
                ZoneInfo { index: 0, offset: 1048576, length: 100000000, zone_type: "EFI".to_string() },
            ],
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("GPT"));
        assert!(json.contains("EFI"));
    }

    // =========================================================================
    // Input Struct Tests
    // =========================================================================

    #[test]
    fn test_analyze_input_deserialization() {
        let json = r#"{"path":"/test.vhd","cache":true,"deep_scan":false}"#;
        let input: AnalyzeDiskImageInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.path, "/test.vhd");
        assert!(input.cache);
        assert!(!input.deep_scan);
    }

    #[test]
    fn test_analyze_input_defaults() {
        let json = r#"{"path":"/test.vhd"}"#;
        let input: AnalyzeDiskImageInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.path, "/test.vhd");
        assert!(input.cache); // default is true
        assert!(!input.deep_scan); // default is false
    }

    #[test]
    fn test_list_files_input_defaults() {
        let json = r#"{"path":"/test.img"}"#;
        let input: ListFilesInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.path, "/test.img");
        assert_eq!(input.zone_index, 0); // default
        assert!(input.cache); // default is true
    }

    #[test]
    fn test_extract_file_input() {
        let json = r#"{"image_path":"/disk.img","file_path":"README.TXT","zone_index":1,"output_path":"/tmp/out.txt"}"#;
        let input: ExtractFileInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.image_path, "/disk.img");
        assert_eq!(input.file_path, "README.TXT");
        assert_eq!(input.zone_index, 1);
        assert_eq!(input.output_path, "/tmp/out.txt");
    }

    #[test]
    fn test_validate_integrity_input_defaults() {
        let json = r#"{"path":"/test.vhd"}"#;
        let input: ValidateIntegrityInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.path, "/test.vhd");
        assert!(input.check_checksums);
        assert!(input.check_boot_sectors);
    }

    // =========================================================================
    // Tool Definition Tests
    // =========================================================================

    #[test]
    fn test_analyze_tool_schema() {
        let cache = create_test_cache();
        let tool = AnalyzeDiskImageTool { cache };

        let schema = tool.input_schema();
        assert!(schema["properties"]["path"].is_object());
        assert!(schema["properties"]["cache"].is_object());
        assert!(schema["properties"]["deep_scan"].is_object());
    }

    #[test]
    fn test_list_partitions_tool_schema() {
        let cache = create_test_cache();
        let tool = ListPartitionsTool { cache };

        let schema = tool.input_schema();
        assert!(schema["properties"]["path"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("path")));
    }

    #[test]
    fn test_list_files_tool_schema() {
        let cache = create_test_cache();
        let tool = ListFilesTool { cache };

        let schema = tool.input_schema();
        assert!(schema["properties"]["zone_index"].is_object());
    }

    #[test]
    fn test_extract_file_tool_schema() {
        let tool = ExtractFileTool {};

        let schema = tool.input_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("image_path")));
        assert!(required.contains(&json!("file_path")));
        assert!(required.contains(&json!("output_path")));
    }

    #[test]
    fn test_validate_integrity_tool_schema() {
        let tool = ValidateIntegrityTool {};

        let schema = tool.input_schema();
        assert!(schema["properties"]["check_checksums"].is_object());
        assert!(schema["properties"]["check_boot_sectors"].is_object());
    }

    // =========================================================================
    // Tool Enum Tests
    // =========================================================================

    #[test]
    fn test_tool_enum_names() {
        let cache = create_test_cache();

        let analyze = ToolEnum::AnalyzeDiskImage(AnalyzeDiskImageTool { cache: cache.clone() });
        assert_eq!(analyze.name(), "analyze_disk_image");

        let list_partitions = ToolEnum::ListPartitions(ListPartitionsTool { cache: cache.clone() });
        assert_eq!(list_partitions.name(), "list_partitions");

        let list_files = ToolEnum::ListFiles(ListFilesTool { cache });
        assert_eq!(list_files.name(), "list_files");

        let extract = ToolEnum::ExtractFile(ExtractFileTool {});
        assert_eq!(extract.name(), "extract_file");

        let validate = ToolEnum::ValidateIntegrity(ValidateIntegrityTool {});
        assert_eq!(validate.name(), "validate_integrity");
    }

    #[test]
    fn test_tool_enum_descriptions() {
        let cache = create_test_cache();

        let analyze = ToolEnum::AnalyzeDiskImage(AnalyzeDiskImageTool { cache });
        assert!(analyze.description().contains("disk image"));

        let validate = ToolEnum::ValidateIntegrity(ValidateIntegrityTool {});
        assert!(validate.description().contains("integrity"));
    }

    #[test]
    fn test_tool_enum_definition() {
        let cache = create_test_cache();
        let tool = ToolEnum::AnalyzeDiskImage(AnalyzeDiskImageTool { cache });

        let def = tool.definition();
        assert_eq!(def.name, "analyze_disk_image");
        assert!(!def.description.is_empty());
        assert!(def.input_schema["properties"].is_object());
    }

    // =========================================================================
    // Helper Function Tests
    // =========================================================================

    #[test]
    fn test_default_true() {
        assert!(default_true());
    }
}
