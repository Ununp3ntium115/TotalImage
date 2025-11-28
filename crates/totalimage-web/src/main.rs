//! TotalImage Web - REST API server for disk image analysis
//!
//! Provides HTTP API endpoints for vault inspection, zone enumeration,
//! and filesystem analysis.

mod cache;

use axum::{
    extract::{Query, State},
    http::{header, Method, StatusCode},
    middleware,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use cache::MetadataCache;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Seek, SeekFrom};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use totalimage_core::{validate_file_path, Result as TotalImageResult, Territory, ZoneTable};
use totalimage_mcp::{auth_middleware, AuthConfig};
use totalimage_territories::{FatTerritory, IsoTerritory, NtfsTerritory};
use totalimage_vaults::{open_vault, VaultConfig};
use totalimage_zones::{GptZoneTable, MbrZoneTable};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

/// Application version
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Shared application state
#[derive(Clone)]
struct AppState {
    cache: Arc<MetadataCache>,
    start_time: std::time::Instant,
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

    let state = AppState {
        cache,
        start_time: std::time::Instant::now(),
    };

    // Configure authentication
    let auth_config = web_auth_config();
    let auth_enabled = auth_config.enabled;
    let auth_config = Arc::new(auth_config);

    if auth_enabled {
        tracing::info!("Authentication enabled");
        if !auth_config.api_keys.is_empty() {
            tracing::info!("  API key authentication: {} keys configured", auth_config.api_keys.len());
        }
        if auth_config.jwt_secret.is_some() || auth_config.jwt_public_key.is_some() {
            tracing::info!("  JWT authentication: enabled");
        }
    } else {
        tracing::warn!("Authentication disabled - all endpoints are public");
    }

    // Configure CORS policy
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_origin(Any); // In production, restrict to specific origins

    // Configure request timeout (30 seconds)
    let timeout = TimeoutLayer::new(Duration::from_secs(30));

    // Build middleware stack
    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(timeout)
        .layer(cors);

    // Build protected routes (require auth)
    let protected_routes = Router::new()
        .route("/api/vault/info", get(vault_info))
        .route("/api/vault/zones", get(vault_zones))
        .route("/api/vault/files", get(vault_files))
        .route_layer(middleware::from_fn_with_state(auth_config.clone(), auth_middleware));

    // Build application routes
    let app = Router::new()
        // Public routes (no auth required)
        .route("/health", get(health))
        .route("/api/status", get(status))
        // Protected routes
        .merge(protected_routes)
        // Apply middleware and state
        .layer(middleware)
        .with_state(state);

    // Configure server address from environment
    let addr: SocketAddr = std::env::var("TOTALIMAGE_WEB_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_string())
        .parse()
        .unwrap_or_else(|e| {
            tracing::error!("Invalid TOTALIMAGE_WEB_ADDR: {}", e);
            std::process::exit(1);
        });

    // Check for TLS configuration
    let tls_cert = std::env::var("TOTALIMAGE_TLS_CERT").ok();
    let tls_key = std::env::var("TOTALIMAGE_TLS_KEY").ok();
    let use_tls = tls_cert.is_some() && tls_key.is_some();

    let protocol = if use_tls { "https" } else { "http" };
    tracing::info!("TotalImage Web Server v{} starting on {}://{}", VERSION, protocol, addr);
    println!("TotalImage Web Server v{}", VERSION);
    println!("   Listening on {}://{}", protocol, addr);
    if use_tls {
        println!("   TLS enabled");
    }
    println!();
    println!("   Endpoints:");
    println!("   - GET  /health                              Health check");
    println!("   - GET  /api/status                          Detailed status");
    println!("   - GET  /api/vault/info?path=<image_file>    Vault metadata");
    println!("   - GET  /api/vault/zones?path=<image_file>   Partition listing");
    println!("   - GET  /api/vault/files?path=<img>&zone=N   File listing");

    if use_tls {
        // Start server with TLS
        let cert_path = PathBuf::from(tls_cert.unwrap());
        let key_path = PathBuf::from(tls_key.unwrap());

        let tls_config = match RustlsConfig::from_pem_file(&cert_path, &key_path).await {
            Ok(config) => config,
            Err(e) => {
                tracing::error!("Failed to load TLS certificates: {}", e);
                eprintln!("Error: Failed to load TLS certificates: {}", e);
                eprintln!("Hint: Check that TOTALIMAGE_TLS_CERT and TOTALIMAGE_TLS_KEY point to valid PEM files");
                std::process::exit(1);
            }
        };

        tracing::info!("TLS configured with cert: {}, key: {}", cert_path.display(), key_path.display());

        let server = axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service());

        // Note: axum-server doesn't support graceful shutdown the same way,
        // but we handle SIGTERM at the process level
        if let Err(e) = server.await {
            tracing::error!("Server error: {}", e);
            std::process::exit(1);
        }
    } else {
        // Start server without TLS
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("Failed to bind to {}: {}", addr, e);
                eprintln!("Error: Failed to bind to {}: {}", addr, e);
                eprintln!("Hint: Check if the port is already in use or if you have permission to bind");
                std::process::exit(1);
            }
        };

        // Run server with graceful shutdown
        let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

        if let Err(e) = server.await {
            tracing::error!("Server error: {}", e);
            std::process::exit(1);
        }
    }

    tracing::info!("Server shutdown complete");
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown...");
        }
    }
}

/// Health check endpoint - simple response for load balancers
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy"
    }))
}

/// Detailed status endpoint
async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let uptime = state.start_time.elapsed();
    let cache_stats = state.cache.stats().ok();

    Json(serde_json::json!({
        "status": "healthy",
        "version": VERSION,
        "uptime_seconds": uptime.as_secs(),
        "cache": cache_stats.map(|s| serde_json::json!({
            "vault_info_count": s.vault_info_count,
            "zone_table_count": s.zone_table_count,
            "dir_listings_count": s.dir_listings_count,
            "estimated_size_bytes": s.estimated_size_bytes
        }))
    }))
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
    let mut vault = open_vault(&path, VaultConfig::default())?;

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
    let mut vault = open_vault(&path, VaultConfig::default())?;

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

/// Query parameters for file listing
#[derive(Deserialize)]
struct FilesQuery {
    path: String,
    #[serde(default)]
    zone: Option<usize>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    100
}

/// File listing response
#[derive(Serialize, Deserialize, Clone)]
struct VaultFilesResponse {
    path: String,
    zone_index: usize,
    filesystem_type: String,
    total_files: usize,
    offset: usize,
    limit: usize,
    files: Vec<FileInfo>,
}

#[derive(Serialize, Deserialize, Clone)]
struct FileInfo {
    name: String,
    path: String,
    size: u64,
    is_directory: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
}

/// GET /api/vault/files?path=<image_file>&zone=<index>
async fn vault_files(
    State(state): State<AppState>,
    Query(params): Query<FilesQuery>,
) -> impl IntoResponse {
    let zone_index = params.zone.unwrap_or(0);
    let cache_key = format!("{}:zone{}:{}:{}", params.path, zone_index, params.offset, params.limit);

    // Check cache first
    if let Ok(Some(cached_files)) = state.cache.get_dir_listing::<VaultFilesResponse>(&cache_key) {
        tracing::info!("Cache HIT for files: {}", cache_key);
        return (StatusCode::OK, Json(cached_files)).into_response();
    }

    tracing::info!("Cache MISS for files: {}", cache_key);

    // Get file listing
    match get_vault_files(&params.path, zone_index, params.offset, params.limit) {
        Ok(files) => {
            // Store in cache
            if let Err(e) = state.cache.set_dir_listing(&cache_key, &files) {
                tracing::warn!("Failed to cache files: {}", e);
            }
            (StatusCode::OK, Json(files)).into_response()
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

fn get_vault_files(
    image_path: &str,
    zone_index: usize,
    offset: usize,
    limit: usize,
) -> TotalImageResult<VaultFilesResponse> {
    // Validate path to prevent path traversal attacks
    let path = validate_file_path(image_path)?;
    let mut vault = open_vault(&path, VaultConfig::default())?;

    let sector_size = 512;

    // Find the zone - clone to avoid lifetime issues
    let zone = if let Ok(mbr) = MbrZoneTable::parse(vault.content(), sector_size) {
        mbr.enumerate_zones().get(zone_index).cloned()
    } else if let Ok(gpt) = GptZoneTable::parse(vault.content(), sector_size) {
        gpt.enumerate_zones().get(zone_index).cloned()
    } else {
        None
    };

    let zone = zone.ok_or_else(|| {
        totalimage_core::Error::invalid_operation(format!("Zone {} not found", zone_index))
    })?;

    // Try to parse the zone as different filesystem types
    // First, read the zone data into owned buffer
    let zone_data = {
        let read_size = zone.length.min(64 * 1024 * 1024) as usize; // Max 64MB for initial read
        let mut data = vec![0u8; read_size];
        vault.content().seek(SeekFrom::Start(zone.offset))?;
        vault.content().read_exact(&mut data)?;
        data
    };

    // Try FAT first (most common)
    {
        let mut cursor = Cursor::new(&zone_data[..]);
        if let Ok(fat) = FatTerritory::parse(&mut cursor) {
            let fs_type = fat.identify().to_string();
            cursor.seek(SeekFrom::Start(0))?;
            if let Ok(entries) = fat.list_directory(&mut cursor, "/") {
                return build_files_response(image_path, zone_index, fs_type, entries, offset, limit);
            }
        }
    }

    // Try NTFS - needs owned cursor since NtfsTerritory takes ownership
    {
        let cursor = Cursor::new(zone_data.clone());
        if let Ok(mut ntfs) = NtfsTerritory::parse(cursor) {
            let fs_type = ntfs.identify().to_string();
            if let Ok(entries) = ntfs.read_directory_at_path("/") {
                return build_files_response(image_path, zone_index, fs_type, entries, offset, limit);
            }
        }
    }

    // Try ISO - ISO needs DirectoryRecord, use root directory
    {
        let mut cursor = Cursor::new(&zone_data[..]);
        if let Ok(iso) = IsoTerritory::parse(&mut cursor) {
            let fs_type = iso.identify().to_string();
            // Read root directory entries
            cursor.seek(SeekFrom::Start(0))?;
            let root_record = iso.primary_descriptor().root_directory_record.clone();
            if let Ok(dir_entries) = iso.read_directory(&mut cursor, &root_record) {
                // Convert DirectoryRecord to OccupantInfo
                let entries: Vec<totalimage_core::OccupantInfo> = dir_entries
                    .into_iter()
                    .map(|rec| totalimage_core::OccupantInfo {
                        name: rec.file_name(),
                        is_directory: rec.is_directory(),
                        size: rec.data_length.get() as u64,
                        created: None,
                        modified: None,
                        accessed: None,
                        attributes: rec.file_flags as u32,
                    })
                    .collect();
                return build_files_response(image_path, zone_index, fs_type, entries, offset, limit);
            }
        }
    }

    // No recognized filesystem
    Ok(VaultFilesResponse {
        path: image_path.to_string(),
        zone_index,
        filesystem_type: "Unknown".to_string(),
        total_files: 0,
        offset,
        limit,
        files: vec![],
    })
}

fn build_files_response(
    image_path: &str,
    zone_index: usize,
    fs_type: String,
    entries: Vec<totalimage_core::OccupantInfo>,
    offset: usize,
    limit: usize,
) -> TotalImageResult<VaultFilesResponse> {
    let total_files = entries.len();

    let files: Vec<FileInfo> = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|entry| {
            // Construct path from name (entries are at root level)
            let path = format!("/{}", entry.name);
            FileInfo {
                name: entry.name.clone(),
                path,
                size: entry.size,
                is_directory: entry.is_directory,
                modified: entry.modified.map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            }
        })
        .collect();

    Ok(VaultFilesResponse {
        path: image_path.to_string(),
        zone_index,
        filesystem_type: fs_type,
        total_files,
        offset,
        limit,
        files,
    })
}

/// Create AuthConfig from web-specific environment variables
///
/// Uses TOTALIMAGE_WEB_ prefix for configuration:
/// - TOTALIMAGE_WEB_AUTH_ENABLED: Enable authentication (true/false)
/// - TOTALIMAGE_WEB_JWT_SECRET: JWT secret key for HMAC algorithms
/// - TOTALIMAGE_WEB_JWT_PUBLIC_KEY: JWT public key for RSA/EC algorithms
/// - TOTALIMAGE_WEB_JWT_ALGORITHM: JWT algorithm (HS256, RS256, etc.)
/// - TOTALIMAGE_WEB_API_KEYS: Comma-separated list of API keys
/// - TOTALIMAGE_WEB_JWT_ISSUER: Expected JWT issuer
/// - TOTALIMAGE_WEB_JWT_AUDIENCE: Expected JWT audience
fn web_auth_config() -> AuthConfig {
    use jsonwebtoken::Algorithm;

    let enabled = std::env::var("TOTALIMAGE_WEB_AUTH_ENABLED")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    let jwt_secret = std::env::var("TOTALIMAGE_WEB_JWT_SECRET").ok();
    let jwt_public_key = std::env::var("TOTALIMAGE_WEB_JWT_PUBLIC_KEY").ok();

    let jwt_algorithm = std::env::var("TOTALIMAGE_WEB_JWT_ALGORITHM")
        .ok()
        .and_then(|alg| match alg.to_uppercase().as_str() {
            "HS256" => Some(Algorithm::HS256),
            "HS384" => Some(Algorithm::HS384),
            "HS512" => Some(Algorithm::HS512),
            "RS256" => Some(Algorithm::RS256),
            "RS384" => Some(Algorithm::RS384),
            "RS512" => Some(Algorithm::RS512),
            "ES256" => Some(Algorithm::ES256),
            "ES384" => Some(Algorithm::ES384),
            _ => None,
        })
        .unwrap_or(Algorithm::HS256);

    let api_keys: Vec<String> = std::env::var("TOTALIMAGE_WEB_API_KEYS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let jwt_issuer = std::env::var("TOTALIMAGE_WEB_JWT_ISSUER").ok();
    let jwt_audience = std::env::var("TOTALIMAGE_WEB_JWT_AUDIENCE").ok();

    AuthConfig {
        enabled,
        jwt_secret,
        jwt_public_key,
        jwt_algorithm,
        api_keys,
        jwt_issuer,
        jwt_audience,
    }
}
