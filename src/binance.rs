use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Binance REST API client
pub struct BinanceClient {
    client: Client,
    base_url: String,
}

/// Symbol information from Binance API (simplified)
#[derive(Debug, Deserialize, Serialize)]
pub struct SymbolInfo {
    pub symbol: String,
    #[serde(rename = "baseAsset")]
    pub base_asset: String,
    #[serde(rename = "quoteAsset")]  
    pub quote_asset: String,
    pub status: String,
    #[serde(default = "default_precision")]
    pub price_precision: u32,
    #[serde(skip)]
    pub base_asset_precision: u32,
    #[serde(skip)]
    pub quote_precision: u32,
    #[serde(skip)]
    pub quantity_precision: u32,
    #[serde(skip)]
    pub filters: Vec<SymbolFilter>,
}

fn default_precision() -> u32 {
    8
}

/// Symbol filters (simplified - we'll skip these for now)
#[derive(Debug, Deserialize, Serialize)]
pub struct SymbolFilter {
    #[serde(rename = "filterType")]
    pub filter_type: String,
}

/// Exchange information response
#[derive(Debug, Deserialize)]
struct ExchangeInfo {
    symbols: Vec<SymbolInfo>,
}

/// WebSocket stream configuration
#[derive(Debug, Serialize)]
pub struct StreamConfig {
    pub method: String,
    pub params: Vec<String>,
    pub id: u64,
}

impl BinanceClient {
    /// Create a new Binance client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://api.binance.com".to_string(),
        }
    }
    
    /// Get symbol information
    pub async fn get_symbol_info(&self, symbol: &str) -> Result<SymbolInfo> {
        let url = format!("{}/api/v3/exchangeInfo", self.base_url);
        
        debug!("Fetching exchange info from: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("API request failed with status: {}", response.status()));
        }
        
        let exchange_info: ExchangeInfo = response.json().await?;
        
        let symbol_info = exchange_info
            .symbols
            .into_iter()
            .find(|s| s.symbol.to_uppercase() == symbol.to_uppercase())
            .ok_or_else(|| anyhow!("Symbol {} not found", symbol))?;
            
        Ok(symbol_info)
    }
    
    /// Get current server time (for connection testing)
    pub async fn get_server_time(&self) -> Result<u64> {
        let url = format!("{}/api/v3/time", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Server time request failed: {}", response.status()));
        }
        
        let time_response: serde_json::Value = response.json().await?;
        let server_time = time_response["serverTime"]
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid server time response"))?;
            
        Ok(server_time)
    }
    
    /// Generate WebSocket stream URL for order book
    pub fn get_orderbook_stream_url(&self, symbol: &str) -> String {
        let stream_name = format!("{}@depth", symbol.to_lowercase());
        format!("wss://stream.binance.com:9443/ws/{}", stream_name)
    }
    
    /// Generate WebSocket stream URL for order book updates (faster)
    pub fn get_orderbook_diff_stream_url(&self, symbol: &str) -> String {
        let stream_name = format!("{}@depth@100ms", symbol.to_lowercase());
        format!("wss://stream.binance.com:9443/ws/{}", stream_name)
    }
}

impl Default for BinanceClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_get_symbol_info() {
        let client = BinanceClient::new();
        let result = client.get_symbol_info("BTCUSDT").await;
        
        assert!(result.is_ok());
        let symbol_info = result.unwrap();
        assert_eq!(symbol_info.symbol, "BTCUSDT");
        assert_eq!(symbol_info.base_asset, "BTC");
        assert_eq!(symbol_info.quote_asset, "USDT");
    }
    
    #[tokio::test]
    async fn test_get_server_time() {
        let client = BinanceClient::new();
        let result = client.get_server_time().await;
        
        assert!(result.is_ok());
        let server_time = result.unwrap();
        assert!(server_time > 0);
    }
    
    #[test]
    fn test_stream_urls() {
        let client = BinanceClient::new();
        
        let url = client.get_orderbook_stream_url("BTCUSDT");
        assert!(url.contains("btcusdt@depth"));
        
        let diff_url = client.get_orderbook_diff_stream_url("BTCUSDT");
        assert!(diff_url.contains("btcusdt@depth@100ms"));
    }
}