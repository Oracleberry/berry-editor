//! BerryCode Web Server
//!
//! Web-based interface for BerryCode AI pair programming

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "berrycode-web")]
#[command(about = "BerryCode Web Server - AI pair programming in your browser")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "7778")]
    port: u16,

    /// Templates directory
    #[arg(short, long)]
    templates: Option<PathBuf>,

    /// Database URL (defaults to sqlite:data/berrycode.db)
    #[arg(short, long, env = "DATABASE_URL", default_value = "sqlite:data/berrycode.db")]
    database_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging first
    tracing_subscriber::fmt::init();

    // Load .env file
    match dotenv::dotenv() {
        Ok(path) => tracing::info!("Loaded .env from: {:?}", path),
        Err(e) => tracing::warn!("Failed to load .env file: {}", e),
    }

    // Parse arguments
    let args = Args::parse();

    tracing::info!("Starting BerryCode Web Server");
    tracing::info!("Database: {}", args.database_url);
    tracing::info!("Port: {}", args.port);

    // Check API configuration
    let has_openai = std::env::var("OPENAI_API_KEY").is_ok();
    let has_anthropic = std::env::var("ANTHROPIC_API_KEY").is_ok();

    if has_openai {
        tracing::info!("✓ OPENAI_API_KEY is configured");
        if let Ok(api_base) = std::env::var("OPENAI_API_BASE") {
            tracing::info!("  API Base: {}", api_base);
        }
    }
    if has_anthropic {
        tracing::info!("✓ ANTHROPIC_API_KEY is configured");
    }
    if !has_openai && !has_anthropic {
        tracing::warn!("⚠ No API keys found. Chat functionality will not work.");
    }

    // Check model configuration
    if let Ok(model) = std::env::var("BERRYCODE_MODEL").or_else(|_| std::env::var("MODEL")) {
        tracing::info!("✓ Model: {}", model);
    } else {
        tracing::info!("  Using default model");
    }

    // Run server
    berrycode::web::run_server(args.port, args.templates, args.database_url).await?;

    Ok(())
}
