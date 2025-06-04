use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Trading symbol (e.g., BTCUSDT)
    pub symbol: String,
    
    /// Fair price calculation method
    pub calculation_method: FairPriceMethod,
    
    /// WebSocket configuration
    pub websocket: WebSocketConfig,
    
    /// Order book configuration
    pub order_book: OrderBookConfig,
}

/// Fair price calculation methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FairPriceMethod {
    /// Simple mid-price: (best_bid + best_ask) / 2
    MidPrice,
    
    /// Volume-weighted average price of top N levels
    VolumeWeighted { levels: usize },
    
    /// Micro-price considering order flow
    MicroPrice,
}

/// WebSocket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Binance WebSocket base URL
    pub base_url: String,
    
    /// Reconnection settings
    pub reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
    
    /// Heartbeat settings
    pub ping_interval_ms: u64,
}

/// Order book configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookConfig {
    /// Maximum depth to maintain
    pub max_depth: usize,
    
    /// Update frequency threshold (microseconds)
    pub update_threshold_us: u64,
}

impl Config {
    pub fn new(symbol: String, method_str: String) -> Self {
        let calculation_method = match method_str.to_lowercase().as_str() {
            "mid-price" => FairPriceMethod::MidPrice,
            "volume-weighted" => FairPriceMethod::VolumeWeighted { levels: 5 },
            "micro-price" => FairPriceMethod::MicroPrice,
            _ => FairPriceMethod::MidPrice,
        };
        
        Self {
            symbol,
            calculation_method,
            websocket: WebSocketConfig {
                base_url: "wss://stream.binance.com:9443/ws/".to_string(),
                reconnect_attempts: 5,
                reconnect_delay_ms: 1000,
                ping_interval_ms: 30000,
            },
            order_book: OrderBookConfig {
                max_depth: 100,
                update_threshold_us: 1000, // 1ms
            },
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new("BTCUSDT".to_string(), "mid-price".to_string())
    }
}

impl std::fmt::Display for FairPriceMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FairPriceMethod::MidPrice => write!(f, "Mid-Price"),
            FairPriceMethod::VolumeWeighted { levels } => {
                write!(f, "Volume-Weighted (top {} levels)", levels)
            }
            FairPriceMethod::MicroPrice => write!(f, "Micro-Price"),
        }
    }
}