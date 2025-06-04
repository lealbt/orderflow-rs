use crate::binance::BinanceClient;
use crate::config::Config;
use crate::fair_price::FairPriceCalculator;
use crate::order_book::{OrderBookManager, OrderBookUpdate};
use anyhow::{Result, anyhow};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// WebSocket connection manager
pub struct WebSocketManager {
    config: Config,
    order_book_manager: Arc<OrderBookManager>,
    fair_price_calculator: Arc<FairPriceCalculator>,
    binance_client: BinanceClient,
}

impl WebSocketManager {
    pub fn new(
        config: Config,
        order_book_manager: Arc<OrderBookManager>,
        fair_price_calculator: Arc<FairPriceCalculator>,
    ) -> Self {
        Self {
            config,
            order_book_manager,
            fair_price_calculator,
            binance_client: BinanceClient::new(),
        }
    }
    
    /// Start WebSocket connection and processing
    pub async fn start(&self) -> Result<()> {
        let mut reconnect_attempts = 0;
        let max_attempts = self.config.websocket.reconnect_attempts;
        
        while reconnect_attempts < max_attempts {
            match self.connect_and_process().await {
                Ok(_) => {
                    info!("WebSocket connection completed successfully");
                    break;
                }
                Err(e) => {
                    reconnect_attempts += 1;
                    error!(
                        "WebSocket connection failed (attempt {}/{}): {}",
                        reconnect_attempts, max_attempts, e
                    );
                    
                    if reconnect_attempts < max_attempts {
                        info!("Retrying in {} seconds...", 
                              self.config.websocket.reconnect_delay_ms / 1000);
                        tokio::time::sleep(Duration::from_millis(
                            self.config.websocket.reconnect_delay_ms
                        )).await;
                    } else {
                        return Err(anyhow!("Max reconnection attempts reached"));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Connect to WebSocket and process messages
    async fn connect_and_process(&self) -> Result<()> {
        // Get order book snapshot first for initialization
        info!("📊 Fetching initial order book snapshot...");
        self.initialize_order_book().await?;
        
        // Connect to WebSocket stream
        let stream_url = self.binance_client.get_orderbook_diff_stream_url(&self.config.symbol);
        info!("🔗 Connecting to WebSocket: {}", stream_url);
        
        let (ws_stream, _response) = connect_async(&stream_url).await?;
        info!("✅ WebSocket connected successfully");
        
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Set up ping interval for connection health
        let mut ping_interval = interval(Duration::from_millis(
            self.config.websocket.ping_interval_ms
        ));
        
        // Message processing loop
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Err(e) = self.process_message(&text).await {
                                warn!("Failed to process message: {}", e);
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            debug!("Received ping, sending pong");
                            if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                                error!("Failed to send pong: {}", e);
                                break;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {
                            debug!("Received pong");
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("WebSocket connection closed by server");
                            break;
                        }
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            warn!("WebSocket stream ended");
                            break;
                        }
                        _ => {}
                    }
                }
                
                // Send periodic pings
                _ = ping_interval.tick() => {
                    debug!("Sending ping");
                    if let Err(e) = ws_sender.send(Message::Ping(vec![])).await {
                        error!("Failed to send ping: {}", e);
                        break;
                    }
                }
            }
        }
        
        Err(anyhow!("WebSocket connection ended"))
    }
    
    /// Initialize order book from REST API snapshot
    async fn initialize_order_book(&self) -> Result<()> {
        let snapshot_url = format!(
            "https://api.binance.com/api/v3/depth?symbol={}&limit=100",
            self.config.symbol
        );
        
        let client = reqwest::Client::new();
        let response = client.get(&snapshot_url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch order book snapshot: {}", response.status()));
        }
        
        let snapshot: crate::order_book::OrderBookSnapshot = response.json().await?;
        
        self.order_book_manager
            .initialize_from_snapshot(&self.config.symbol, snapshot)?;
            
        info!("✅ Order book initialized with {} bids and {} asks", 
              self.order_book_manager.get_order_book().map_or(0, |ob| ob.bids.len()),
              self.order_book_manager.get_order_book().map_or(0, |ob| ob.asks.len()));
        
        Ok(())
    }
    
    /// Process incoming WebSocket message
    async fn process_message(&self, message: &str) -> Result<()> {
        // Parse the JSON message
        let json_value: Value = serde_json::from_str(message)?;
        
        // Check if it's a depth update
        if json_value.get("e").and_then(|v| v.as_str()) == Some("depthUpdate") {
            let update: OrderBookUpdate = serde_json::from_str(message)?;
            
            // Verify symbol matches
            if update.symbol != self.config.symbol {
                warn!("Received update for wrong symbol: {}", update.symbol);
                return Ok(());
            }
            
            // Apply the update
            self.order_book_manager.apply_update(update)?;
            
            // Calculate and display fair price
            self.calculate_and_display_fair_price().await?;
        }
        
        Ok(())
    }
    
    /// Calculate fair price and display results
    async fn calculate_and_display_fair_price(&self) -> Result<()> {
        if !self.order_book_manager.is_ready() {
            return Ok(()); // Skip if order book not ready
        }
        
        let order_book = match self.order_book_manager.get_order_book() {
            Some(ob) => ob,
            None => return Ok(()),
        };
        
        // We need to handle the Arc<FairPriceCalculator> properly
        // Since it needs to be mutable, we'll create a temporary calculator
        let mut temp_calculator = FairPriceCalculator::new(
            self.fair_price_calculator.get_method().clone()
        );
        
        let fair_price_result = match temp_calculator.calculate(&order_book) {
            Some(result) => result,
            None => {
                warn!("Failed to calculate fair price");
                return Ok(());
            }
        };
        
        // Display the results
        self.display_results(&fair_price_result, &order_book).await;
        
        Ok(())
    }
    
    /// Display calculation results
    async fn display_results(
        &self,
        result: &crate::fair_price::FairPriceResult,
        order_book: &crate::order_book::OrderBook,
    ) {
        let best_bid = order_book.best_bid().map(|b| b.price.0).unwrap_or(0.0);
        let best_ask = order_book.best_ask().map(|a| a.price.0).unwrap_or(0.0);
        
        // Create a formatted output
        let output = format!(
            "\n┌─ {} Fair Price Update ─────────────────────────────────┐\n\
             │ Fair Price: ${:<15.4} Method: {:<20} │\n\
             │ Mid Price:  ${:<15.4} Confidence: {:<17.1}% │\n\
             │ Best Bid:   ${:<15.4} Best Ask: ${:<16.4} │\n\
             │ Spread:     ${:<15.4} ({:<20.3}%) │\n\
             │ Signal:     {:<35} │\n\
             │ Volumes:    Bid: {:<8.2} Ask: {:<8.2} Total: {:<8.2} │\n\
             │ Flow:       {:<35.2} │\n\
             └─────────────────────────────────────────────────────────┘",
            self.config.symbol,
            result.fair_price,
            result.calculation_method,
            result.mid_price,
            result.confidence * 100.0,
            best_bid,
            best_ask,
            result.spread,
            (result.spread / result.mid_price) * 100.0,
            result.market_signal(),
            result.metadata.bid_volume,
            result.metadata.ask_volume,
            result.metadata.total_volume,
            result.metadata.order_flow_imbalance,
        );
        
        info!("{}", output);
        
        // Log additional debug information
        debug!(
            "Order book stats - Bids: {}, Asks: {}, Last update: {}",
            order_book.bids.len(),
            order_book.asks.len(),
            order_book.last_update
        );
    }
    
    /// Health check for WebSocket connection
    pub async fn health_check(&self) -> Result<bool> {
        // Try to get server time from Binance API
        match timeout(
            Duration::from_secs(5),
            self.binance_client.get_server_time()
        ).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(e)) => {
                warn!("Health check failed: {}", e);
                Ok(false)
            }
            Err(_) => {
                warn!("Health check timeout");
                Ok(false)
            }
        }
    }
    
    /// Get connection statistics
    pub fn get_stats(&self) -> ConnectionStats {
        ConnectionStats {
            is_order_book_ready: self.order_book_manager.is_ready(),
            current_spread: self.order_book_manager.get_spread(),
            current_mid_price: self.order_book_manager.get_mid_price(),
            symbol: self.config.symbol.clone(),
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub is_order_book_ready: bool,
    pub current_spread: Option<f64>,
    pub current_mid_price: Option<f64>,
    pub symbol: String,
}

impl std::fmt::Display for ConnectionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Stats for {}: Ready={}, Mid=${:.4}, Spread=${:.4}",
            self.symbol,
            self.is_order_book_ready,
            self.current_mid_price.unwrap_or(0.0),
            self.current_spread.unwrap_or(0.0)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FairPriceMethod;
    
    #[tokio::test]
    async fn test_websocket_manager_creation() {
        let config = Config::new("BTCUSDT".to_string(), "mid-price".to_string());
        let order_book_manager = Arc::new(OrderBookManager::new());
        let fair_price_calculator = Arc::new(FairPriceCalculator::new(FairPriceMethod::MidPrice));
        
        let ws_manager = WebSocketManager::new(
            config,
            order_book_manager,
            fair_price_calculator,
        );
        
        let stats = ws_manager.get_stats();
        assert_eq!(stats.symbol, "BTCUSDT");
        assert!(!stats.is_order_book_ready);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let config = Config::new("BTCUSDT".to_string(), "mid-price".to_string());
        let order_book_manager = Arc::new(OrderBookManager::new());
        let fair_price_calculator = Arc::new(FairPriceCalculator::new(FairPriceMethod::MidPrice));
        
        let ws_manager = WebSocketManager::new(
            config,
            order_book_manager,
            fair_price_calculator,
        );
        
        // This might fail in test environment without internet
        // but should compile and structure correctly
        let _health_result = ws_manager.health_check().await;
    }
}