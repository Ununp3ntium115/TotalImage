//! TotalImage MCP Server - Binary entry point
//!
//! Provides dual-mode operation:
//! - Standalone: stdio transport for Claude Desktop
//! - Integrated: HTTP transport + Fire Marshal registration
//! - Auto: Detect mode based on environment variables

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use totalimage_mcp::{IntegratedConfig, MCPServer, StandaloneConfig};

#[derive(Parser)]
#[command(name = "totalimage-mcp")]
#[command(about = "TotalImage MCP Server - Disk Image Analysis for Claude", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    mode: Option<Mode>,

    /// Cache directory for results
    #[arg(long, global = true)]
    cache_dir: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", global = true)]
    log_level: String,
}

#[derive(Subcommand)]
enum Mode {
    /// Standalone mode (stdio transport for Claude Desktop)
    Standalone {
        /// Configuration file (optional)
        #[arg(long)]
        config: Option<PathBuf>,
    },

    /// Integrated mode (HTTP transport + Fire Marshal registration)
    Integrated {
        /// Fire Marshal URL for registration
        #[arg(long)]
        marshal_url: String,

        /// HTTP server port
        #[arg(long, default_value = "3002")]
        port: u16,

        /// Tool name for registry
        #[arg(long, default_value = "totalimage")]
        tool_name: String,
    },

    /// Auto-detect mode based on environment variables
    Auto,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(cli.log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    // Determine cache directory
    let cache_dir = cli.cache_dir.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(format!("{}/.cache/totalimage", home))
    });

    // Ensure cache directory exists
    std::fs::create_dir_all(&cache_dir)?;

    // Run in appropriate mode
    match cli.mode.unwrap_or(Mode::Auto) {
        Mode::Standalone { config } => {
            run_standalone(cache_dir, config).await?;
        }

        Mode::Integrated {
            marshal_url,
            port,
            tool_name,
        } => {
            run_integrated(cache_dir, marshal_url, port, tool_name).await?;
        }

        Mode::Auto => {
            // Auto-detect based on environment
            if let Ok(marshal_url) = std::env::var("FIRE_MARSHAL_URL") {
                tracing::info!("Auto-detected INTEGRATED mode (FIRE_MARSHAL_URL set)");
                let port = std::env::var("TOTALIMAGE_MCP_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(3002);
                let tool_name = std::env::var("TOTALIMAGE_TOOL_NAME")
                    .unwrap_or_else(|_| "totalimage".to_string());

                run_integrated(cache_dir, marshal_url, port, tool_name).await?;
            } else {
                tracing::info!("Auto-detected STANDALONE mode");
                run_standalone(cache_dir, None).await?;
            }
        }
    }

    Ok(())
}

async fn run_standalone(cache_dir: PathBuf, config_file: Option<PathBuf>) -> Result<()> {
    tracing::info!("Starting TotalImage MCP Server in STANDALONE mode");
    tracing::info!("  Cache directory: {}", cache_dir.display());
    if let Some(config) = &config_file {
        tracing::info!("  Config file: {}", config.display());
    }

    let config = StandaloneConfig {
        cache_dir,
        config_file,
    };

    let server = MCPServer::new_standalone(config)?;
    server.listen_stdio().await?;

    Ok(())
}

async fn run_integrated(
    cache_dir: PathBuf,
    marshal_url: String,
    port: u16,
    tool_name: String,
) -> Result<()> {
    tracing::info!("Starting TotalImage MCP Server in INTEGRATED mode");
    tracing::info!("  Cache directory: {}", cache_dir.display());
    tracing::info!("  Fire Marshal URL: {}", marshal_url);
    tracing::info!("  HTTP Port: {}", port);
    tracing::info!("  Tool Name: {}", tool_name);

    let config = IntegratedConfig {
        cache_dir,
        marshal_url,
        port,
        tool_name,
    };

    let server = Arc::new(MCPServer::new_integrated(config)?);

    // Register with Fire Marshal
    if let Err(e) = server.register_with_marshal().await {
        tracing::warn!("Failed to register with Fire Marshal: {}", e);
        tracing::warn!("Continuing without Fire Marshal registration");
    }

    // Start HTTP server
    server.listen_http().await?;

    Ok(())
}
