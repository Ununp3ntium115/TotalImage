//! TotalImage Web - REST API server for disk image analysis
//!
//! Provides HTTP API endpoints for vault inspection, zone enumeration,
//! and filesystem analysis.

mod cache;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use cache::MetadataCache;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use totalimage_core::{validate_file_path, Result as TotalImageResult, Vault, ZoneTable};
use totalimage_vaults::{RawVault, VaultConfig};
use totalimage_zones::{GptZoneTable, MbrZoneTable};

/// Shared application state
#[derive(Clone)]
struct AppState {
    cache: Arc<MetadataCache>,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize metadata cache
    let cache_dir = std::env::var("TOTALIMAGE_CACHE_DIR")
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            format!("{}/.cache/totalimage", home)
        });
    let cache_path = std::path::PathBuf::from(cache_dir).join("metadata.redb");

    let cache = match MetadataCache::new(cache_path.clone()) {
        Ok(cache) => {
            tracing::info!("Metadata cache initialized at {}", cache_path.display());
            if let Ok(stats) = cache.stats() {
                tracing::info!(
                    "Cache stats: {} vault_info, {} zones, {} dir_listings, ~{} bytes",
                    stats.vault_info_count,
                    stats.zone_table_count,
                    stats.dir_listings_count,
                    stats.estimated_size_bytes
                );
            }
            // TODO: Implement automatic cache maintenance
            // - Spawn background task for periodic cleanup_expired()
            // - Call evict_if_needed() when cache size exceeds MAX_CACHE_SIZE
            // - Consider using tokio::spawn with interval timer
            Arc::new(cache)
        }
        Err(e) => {
            tracing::error!("Failed to initialize cache: {}", e);
            tracing::warn!("Continuing without cache");
            // Create a dummy cache in temp dir
            let temp_path = std::env::temp_dir().join("totalimage_cache.redb");
            Arc::new(MetadataCache::new(temp_path).expect("Failed to create temp cache"))
        }
    };

    let state = AppState { cache };

    // TODO: Production hardening (SEC-007)
    // - Add rate limiting: tower::limit::RateLimitLayer
    // - Add request timeouts: tower::timeout::TimeoutLayer (30s)
    // - Add concurrency limits: tower::limit::ConcurrencyLimitLayer (10)
    // - Configure CORS policy for API access
    // - Add request size limits (10 MB max)
    // - Enable TLS/HTTPS support
    // See: steering/GAP-ANALYSIS.md#SEC-007

    // Build application routes
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/vault/info", get(vault_info))
        .route("/api/vault/zones", get(vault_zones))
        .with_state(state);

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
#[derive(Serialize, Deserialize, Clone)]
struct VaultInfoResponse {
    path: String,
    vault_type: String,
    size_bytes: u64,
    partition_table: Option<PartitionTableInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
struct PartitionTableInfo {
    table_type: String,
    partition_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    disk_signature: Option<String>,
}

/// Zone information response
#[derive(Serialize, Deserialize, Clone)]
struct VaultZonesResponse {
    path: String,
    partition_table: String,
    zones: Vec<ZoneInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ZoneInfo {
    index: usize,
    offset: u64,
    length: u64,
    zone_type: String,
}

/// GET /api/vault/info?path=<image_file>
async fn vault_info(
    State(state): State<AppState>,
    Query(params): Query<VaultQuery>,
) -> impl IntoResponse {
    // Check cache first
    if let Ok(Some(cached_info)) = state.cache.get_vault_info::<VaultInfoResponse>(&params.path) {
        tracing::info!("Cache HIT for vault_info: {}", params.path);
        return (StatusCode::OK, Json(cached_info)).into_response();
    }

    tracing::info!("Cache MISS for vault_info: {}", params.path);

    // Parse vault
    match get_vault_info(&params.path) {
        Ok(info) => {
            // Store in cache
            if let Err(e) = state.cache.set_vault_info(&params.path, &info) {
                tracing::warn!("Failed to cache vault_info: {}", e);
            }
            (StatusCode::OK, Json(info)).into_response()
        }
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
async fn vault_zones(
    State(state): State<AppState>,
    Query(params): Query<VaultQuery>,
) -> impl IntoResponse {
    // Check cache first
    if let Ok(Some(cached_zones)) = state.cache.get_zones::<VaultZonesResponse>(&params.path) {
        tracing::info!("Cache HIT for zones: {}", params.path);
        return (StatusCode::OK, Json(cached_zones)).into_response();
    }

    tracing::info!("Cache MISS for zones: {}", params.path);

    // Parse vault zones
    match get_vault_zones(&params.path) {
        Ok(zones) => {
            // Store in cache
            if let Err(e) = state.cache.set_zones(&params.path, &zones) {
                tracing::warn!("Failed to cache zones: {}", e);
            }
            (StatusCode::OK, Json(zones)).into_response()
        }
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
    // Validate path to prevent path traversal attacks
    let path = validate_file_path(image_path)?;
    let mut vault = RawVault::open(&path, VaultConfig::default())?;

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
    // Validate path to prevent path traversal attacks
    let path = validate_file_path(image_path)?;
    let mut vault = RawVault::open(&path, VaultConfig::default())?;

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
