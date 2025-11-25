//! Fire Marshal CLI - Tool Orchestration for PYRO Platform
//!
//! Provides HTTP API for tool registration and orchestration

use anyhow::Result;
use clap::{Parser, Subcommand};
use fire_marshal::{FireMarshal, FireMarshalConfig};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "fire-marshal")]
#[command(about = "Tool orchestration framework for PYRO Platform Ignition")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Database path
    #[arg(long, env = "FIRE_MARSHAL_DB", default_value = "./fire-marshal.redb")]
    database: PathBuf,

    /// HTTP port
    #[arg(long, env = "FIRE_MARSHAL_PORT", default_value = "3001")]
    port: u16,

    /// Rate limit (requests per second)
    #[arg(long, default_value = "100")]
    rate_limit: u32,

    /// Request timeout in seconds
    #[arg(long, default_value = "30")]
    timeout: u64,

    /// Maximum concurrent requests
    #[arg(long, default_value = "10")]
    max_concurrent: usize,

    /// Log level
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log_level: String,
}

#[derive(Subcommand)]
enum Command {
    /// Start the Fire Marshal server
    Start,

    /// List registered tools (requires running server)
    List {
        /// Fire Marshal URL
        #[arg(long, default_value = "http://localhost:3001")]
        url: String,
    },

    /// Show server statistics (requires running server)
    Stats {
        /// Fire Marshal URL
        #[arg(long, default_value = "http://localhost:3001")]
        url: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(&cli.log_level)
        .init();

    match cli.command.unwrap_or(Command::Start) {
        Command::Start => {
            tracing::info!("Starting Fire Marshal server");
            tracing::info!("  Database: {:?}", cli.database);
            tracing::info!("  Port: {}", cli.port);
            tracing::info!("  Rate limit: {} req/s", cli.rate_limit);
            tracing::info!("  Timeout: {}s", cli.timeout);
            tracing::info!("  Max concurrent: {}", cli.max_concurrent);

            let config = FireMarshalConfig {
                database_path: cli.database,
                port: cli.port,
                rate_limit_rps: cli.rate_limit,
                timeout_secs: cli.timeout,
                max_concurrent: cli.max_concurrent,
            };

            let marshal = FireMarshal::new(config)?;
            marshal.serve().await?;
        }

        Command::List { url } => {
            let response = reqwest::get(format!("{}/tools/list", url))
                .await?
                .json::<serde_json::Value>()
                .await?;

            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        Command::Stats { url } => {
            let response = reqwest::get(format!("{}/stats", url))
                .await?
                .json::<serde_json::Value>()
                .await?;

            println!("{}", serde_json::to_string_pretty(&response)?);
        }
    }

    Ok(())
}
