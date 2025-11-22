//! TotalImage Web - REST API server for disk image analysis
//!
//! Provides HTTP API endpoints for vault inspection, zone enumeration,
//! and filesystem analysis.

use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::Path;
use totalimage_core::{Result as TotalImageResult, Territory, Vault, ZoneTable};
use totalimage_vaults::{RawVault, VaultConfig};
use totalimage_zones::{GptZoneTable, MbrZoneTable};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Build application routes
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/vault/info", get(vault_info))
        .route("/api/vault/zones", get(vault_zones));

    // Run server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("TotalImage Web Server listening on {}", addr);
    println!("🚀 TotalImage Web Server");
    println!("   Listening on http://{}", addr);
    println!();
    println!("   Endpoints:");
    println!("   - GET  /health");
    println!("   - GET  /api/vault/info?path=<image_file>");
    println!("   - GET  /api/vault/zones?path=<image_file>");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Health check endpoint
async fn health() -> &'static str {
    "OK"
}

/// Query parameters for vault endpoints
#[derive(Deserialize)]
struct VaultQuery {
    path: String,
}

/// Vault information response
#[derive(Serialize)]
struct VaultInfoResponse {
    path: String,
    vault_type: String,
    size_bytes: u64,
    partition_table: Option<PartitionTableInfo>,
}

#[derive(Serialize)]
struct PartitionTableInfo {
    table_type: String,
    partition_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk_signature: Option<String>,
}

/// Zone information response
#[derive(Serialize)]
struct VaultZonesResponse {
    path: String,
    partition_table: String,
    zones: Vec<ZoneInfo>,
}

#[derive(Serialize)]
struct ZoneInfo {
    index: usize,
    offset: u64,
    length: u64,
    zone_type: String,
}

/// GET /api/vault/info?path=<image_file>
async fn vault_info(Query(params): Query<VaultQuery>) -> impl IntoResponse {
    match get_vault_info(&params.path) {
        Ok(info) => (StatusCode::OK, Json(info)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// GET /api/vault/zones?path=<image_file>
async fn vault_zones(Query(params): Query<VaultQuery>) -> impl IntoResponse {
    match get_vault_zones(&params.path) {
        Ok(zones) => (StatusCode::OK, Json(zones)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": e.to_string()
            })),
        )
            .into_response(),
    }
}

fn get_vault_info(image_path: &str) -> TotalImageResult<VaultInfoResponse> {
    let path = Path::new(image_path);
    let mut vault = RawVault::open(path, VaultConfig::default())?;

    let vault_type = vault.identify().to_string();
    let size_bytes = vault.length();

    // Try to parse partition table
    let sector_size = 512;
    let partition_table = if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
        Some(PartitionTableInfo {
            table_type: mbr.identify().to_string(),
            partition_count: mbr.enumerate_zones().len(),
            disk_signature: Some(format!("0x{:08X}", mbr.disk_signature())),
        })
    } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
        Some(PartitionTableInfo {
            table_type: gpt.identify().to_string(),
            partition_count: gpt.enumerate_zones().len(),
            disk_signature: None,
        })
    } else {
        None
    };

    Ok(VaultInfoResponse {
        path: image_path.to_string(),
        vault_type,
        size_bytes,
        partition_table,
    })
}

fn get_vault_zones(image_path: &str) -> TotalImageResult<VaultZonesResponse> {
    let path = Path::new(image_path);
    let mut vault = RawVault::open(path, VaultConfig::default())?;

    let sector_size = 512;

    // Try MBR first
    if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
        let zones = mbr
            .enumerate_zones()
            .iter()
            .map(|z| ZoneInfo {
                index: z.index,
                offset: z.offset,
                length: z.length,
                zone_type: z.zone_type.clone(),
            })
            .collect();

        Ok(VaultZonesResponse {
            path: image_path.to_string(),
            partition_table: mbr.identify().to_string(),
            zones,
        })
    } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
        let zones = gpt
            .enumerate_zones()
            .iter()
            .map(|z| ZoneInfo {
                index: z.index,
                offset: z.offset,
                length: z.length,
                zone_type: z.zone_type.clone(),
            })
            .collect();

        Ok(VaultZonesResponse {
            path: image_path.to_string(),
            partition_table: gpt.identify().to_string(),
            zones,
        })
    } else {
        Ok(VaultZonesResponse {
            path: image_path.to_string(),
            partition_table: "None".to_string(),
            zones: Vec::new(),
        })
    }
}
