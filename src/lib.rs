//! # OrderFlow-RS
//! 
//! A high-performance Rust library for real-time fair price calculation
//! from Binance order book data using multiple sophisticated algorithms.
//! 
//! ## Features
//! 
//! - Real-time WebSocket connection to Binance
//! - Multiple fair price calculation methods
//! - Thread-safe order book management
//! - Robust error handling and reconnection logic
//! 
//! ## Quick Start
//! 
//! ```rust,no_run
//! use orderflow_rs::{
//!     Config, BinanceClient, OrderBookManager, FairPriceCalculator, WebSocketManager
//! };
//! use std::sync::Arc;
//! 
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::default();
//!     let order_book_manager = Arc::new(OrderBookManager::new());
//!     let fair_price_calculator = Arc::new(FairPriceCalculator::new(
//!         config.calculation_method.clone()
//!     ));
//!     
//!     let ws_manager = WebSocketManager::new(
//!         config,
//!         order_book_manager,
//!         fair_price_calculator,
//!     );
//!     
//!     ws_manager.start().await?;
//!     Ok(())
//! }
//! ```

pub mod binance;
pub mod config;
pub mod fair_price;
pub mod order_book;
pub mod websocket;

// Re-export main types for easy access
pub use binance::{BinanceClient, SymbolInfo};
pub use config::{Config, FairPriceMethod};
pub use fair_price::{FairPriceCalculator, FairPriceResult, MarketSignal};
pub use order_book::{OrderBook, OrderBookLevel, OrderBookManager, OrderBookUpdate};
pub use websocket::{WebSocketManager, ConnectionStats};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Get library information
pub fn library_info() -> String {
    format!("{} v{}", NAME, VERSION)
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_library_info() {
        let info = library_info();
        assert!(info.contains("orderflow-rs"));
        assert!(info.contains("0.1.0"));
    }
}