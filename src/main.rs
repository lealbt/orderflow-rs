use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{info, warn, error};

mod binance;
mod fair_price;
mod order_book;
mod websocket;
mod config;

use crate::binance::BinanceClient;
use crate::fair_price::FairPriceCalculator;
use crate::order_book::OrderBookManager;
use crate::websocket::WebSocketManager;
use crate::config::Config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Trading symbol (e.g., BTCUSDT)
    #[arg(short, long, default_value = "BTCUSDT")]
    symbol: String,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Fair price calculation method
    #[arg(short, long, default_value = "mid-price")]
    method: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    init_logging(&args.log_level)?;
    
    info!("🚀 Starting OrderFlow-RS - Real-time Fair Price Calculator");
    info!("Symbol: {}", args.symbol);
    info!("Calculation method: {}", args.method);
    
    // Initialize configuration
    let config = Config::new(args.symbol.clone(), args.method);
    
    // Initialize components
    let binance_client = Arc::new(BinanceClient::new());
    let order_book_manager = Arc::new(OrderBookManager::new());
    let fair_price_calculator = Arc::new(FairPriceCalculator::new(config.calculation_method.clone()));
    
    // Try to verify symbol (optional)
    info!("🔍 Attempting to verify symbol {}...", config.symbol);
    match binance_client.get_symbol_info(&config.symbol).await {
        Ok(info) => {
            info!("✅ Symbol {} verified - Base: {}, Quote: {}", 
                  config.symbol, info.base_asset, info.quote_asset);
        }
        Err(e) => {
            warn!("⚠️ Symbol verification failed (continuing anyway): {}", e);
            info!("📡 Proceeding with WebSocket connection...");
        }
    }
    
    // Initialize WebSocket manager
    let ws_manager = WebSocketManager::new(
        config.clone(),
        order_book_manager.clone(),
        fair_price_calculator.clone(),
    );
    
    // Start the WebSocket connection and processing
    match ws_manager.start().await {
        Ok(_) => info!("✅ WebSocket connection established"),
        Err(e) => {
            error!("❌ Failed to start WebSocket: {}", e);
            return Err(e);
        }
    }
    
    // Keep the application running
    info!("🔄 Bot is running... Press Ctrl+C to stop");
    tokio::signal::ctrl_c().await?;
    info!("🛑 Shutting down...");
    
    Ok(())
}

fn init_logging(level: &str) -> Result<()> {
    let filter = match level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    
    tracing_subscriber::fmt()
        .with_max_level(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
    
    Ok(())
}