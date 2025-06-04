use std::collections::BTreeMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Deserialize;
use anyhow::Result;
use tracing::{debug, warn};

/// Ordered float wrapper for price precision
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Price(pub f64);

impl Price {
    pub fn new(value: f64) -> Self {
        if value.is_nan() || value.is_infinite() {
            panic!("Invalid price value: {}", value);
        }
        Price(value)
    }
}

impl Eq for Price {}

impl Ord for Price {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Order book level (price and quantity)
#[derive(Debug, Clone, PartialEq)]
pub struct OrderBookLevel {
    pub price: Price,
    pub quantity: f64,
    pub timestamp: u64,
}

/// Complete order book state
#[derive(Debug, Clone)]
pub struct OrderBook {
    /// Bids (buy orders) - price descending
    pub bids: BTreeMap<Price, OrderBookLevel>,
    /// Asks (sell orders) - price ascending  
    pub asks: BTreeMap<Price, OrderBookLevel>,
    /// Last update timestamp
    pub last_update: u64,
    /// Symbol
    pub symbol: String,
}

/// Order book update from WebSocket
#[derive(Debug, Deserialize)]
pub struct OrderBookUpdate {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id: u64,
    #[serde(rename = "u")]
    pub final_update_id: u64,
    #[serde(rename = "b")]
    pub bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    pub asks: Vec<[String; 2]>,
}

/// Order book snapshot from REST API
#[derive(Debug, Deserialize)]
pub struct OrderBookSnapshot {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

/// Thread-safe order book manager
pub struct OrderBookManager {
    order_book: RwLock<Option<OrderBook>>,
    max_depth: usize,
}

impl OrderBookLevel {
    pub fn new(price: f64, quantity: f64) -> Self {
        Self {
            price: Price::new(price),
            quantity,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
        }
    }
}

impl OrderBook {
    pub fn new(symbol: String) -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            last_update: 0,
            symbol,
        }
    }
    
    /// Get best bid (highest buy price)
    pub fn best_bid(&self) -> Option<&OrderBookLevel> {
        self.bids.values().next_back() // Last element (highest price)
    }
    
    /// Get best ask (lowest sell price)
    pub fn best_ask(&self) -> Option<&OrderBookLevel> {
        self.asks.values().next() // First element (lowest price)
    }
    
    /// Get bid-ask spread
    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask.price.0 - bid.price.0),
            _ => None,
        }
    }
    
    /// Get mid price
    pub fn mid_price(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid.price.0 + ask.price.0) / 2.0),
            _ => None,
        }
    }
    
    /// Get top N levels from each side
    pub fn get_top_levels(&self, n: usize) -> (Vec<&OrderBookLevel>, Vec<&OrderBookLevel>) {
        let top_bids: Vec<&OrderBookLevel> = self.bids
            .values()
            .rev() // Reverse to get highest prices first
            .take(n)
            .collect();
            
        let top_asks: Vec<&OrderBookLevel> = self.asks
            .values()
            .take(n)
            .collect();
            
        (top_bids, top_asks)
    }
    
    /// Check if order book has valid data
    pub fn is_valid(&self) -> bool {
        !self.bids.is_empty() && !self.asks.is_empty() && self.spread().unwrap_or(-1.0) > 0.0
    }
    
    /// Apply order book update
    pub fn apply_update(&mut self, update: &OrderBookUpdate) -> Result<()> {
        // Update bids
        for bid in &update.bids {
            let price = bid[0].parse::<f64>()?;
            let quantity = bid[1].parse::<f64>()?;
            let price_key = Price::new(price);
            
            if quantity == 0.0 {
                // Remove level if quantity is zero
                self.bids.remove(&price_key);
            } else {
                // Update or insert level
                self.bids.insert(price_key, OrderBookLevel::new(price, quantity));
            }
        }
        
        // Update asks
        for ask in &update.asks {
            let price = ask[0].parse::<f64>()?;
            let quantity = ask[1].parse::<f64>()?;
            let price_key = Price::new(price);
            
            if quantity == 0.0 {
                // Remove level if quantity is zero
                self.asks.remove(&price_key);
            } else {
                // Update or insert level
                self.asks.insert(price_key, OrderBookLevel::new(price, quantity));
            }
        }
        
        self.last_update = update.final_update_id;
        
        debug!(
            "Order book updated - Bids: {}, Asks: {}, Spread: {:.4}",
            self.bids.len(),
            self.asks.len(),
            self.spread().unwrap_or(0.0)
        );
        
        Ok(())
    }
}

impl OrderBookManager {
    pub fn new() -> Self {
        Self {
            order_book: RwLock::new(None),
            max_depth: 100,
        }
    }
    
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            order_book: RwLock::new(None),
            max_depth,
        }
    }
    
    /// Initialize order book from snapshot
    pub fn initialize_from_snapshot(&self, symbol: &str, snapshot: OrderBookSnapshot) -> Result<()> {
        let mut order_book = OrderBook::new(symbol.to_string());
        
        // Process bids
        for bid in &snapshot.bids {
            let price = bid[0].parse::<f64>()?;
            let quantity = bid[1].parse::<f64>()?;
            if quantity > 0.0 {
                order_book.bids.insert(
                    Price::new(price), 
                    OrderBookLevel::new(price, quantity)
                );
            }
        }
        
        // Process asks
        for ask in &snapshot.asks {
            let price = ask[0].parse::<f64>()?;
            let quantity = ask[1].parse::<f64>()?;
            if quantity > 0.0 {
                order_book.asks.insert(
                    Price::new(price), 
                    OrderBookLevel::new(price, quantity)
                );
            }
        }
        
        order_book.last_update = snapshot.last_update_id;
        
        // Trim to max depth
        self.trim_to_depth(&mut order_book);
        
        let mut book_guard = self.order_book.write().unwrap();
        *book_guard = Some(order_book);
        
        debug!("Order book initialized from snapshot");
        Ok(())
    }
    
    /// Apply incremental update
    pub fn apply_update(&self, update: OrderBookUpdate) -> Result<()> {
        let mut book_guard = self.order_book.write().unwrap();
        
        match book_guard.as_mut() {
            Some(order_book) => {
                order_book.apply_update(&update)?;
                self.trim_to_depth(order_book);
                Ok(())
            }
            None => {
                warn!("Received update before initialization");
                Err(anyhow::anyhow!("Order book not initialized"))
            }
        }
    }
    
    /// Get current order book snapshot
    pub fn get_order_book(&self) -> Option<OrderBook> {
        let book_guard = self.order_book.read().unwrap();
        book_guard.clone()
    }
    
    /// Get current mid price
    pub fn get_mid_price(&self) -> Option<f64> {
        let book_guard = self.order_book.read().unwrap();
        book_guard.as_ref()?.mid_price()
    }
    
    /// Get current spread
    pub fn get_spread(&self) -> Option<f64> {
        let book_guard = self.order_book.read().unwrap();
        book_guard.as_ref()?.spread()
    }
    
    /// Trim order book to maximum depth
    fn trim_to_depth(&self, order_book: &mut OrderBook) {
        // Keep only top N bids (highest prices)
        if order_book.bids.len() > self.max_depth {
            let keys_to_remove: Vec<Price> = order_book.bids
                .keys()
                .cloned()
                .take(order_book.bids.len() - self.max_depth)
                .collect();
                
            for key in keys_to_remove {
                order_book.bids.remove(&key);
            }
        }
        
        // Keep only top N asks (lowest prices)
        if order_book.asks.len() > self.max_depth {
            let keys_to_remove: Vec<Price> = order_book.asks
                .keys()
                .cloned()
                .skip(self.max_depth)
                .collect();
                
            for key in keys_to_remove {
                order_book.asks.remove(&key);
            }
        }
    }
    
    /// Check if order book is ready
    pub fn is_ready(&self) -> bool {
        let book_guard = self.order_book.read().unwrap();
        book_guard.as_ref().map_or(false, |book| book.is_valid())
    }
}

impl Default for OrderBookManager {
    fn default() -> Self {
        Self::new()
    }
}