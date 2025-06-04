use crate::config::FairPriceMethod;
use crate::order_book::OrderBook;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// Fair price calculation result
#[derive(Debug, Clone)]
pub struct FairPriceResult {
    pub fair_price: f64,
    pub calculation_method: String,
    pub timestamp: u64,
    pub confidence: f64, // 0.0 to 1.0
    pub spread: f64,
    pub mid_price: f64,
    pub metadata: FairPriceMetadata,
}

/// Additional metadata for fair price calculation
#[derive(Debug, Clone)]
pub struct FairPriceMetadata {
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub total_volume: f64,
    pub weighted_bid_price: f64,
    pub weighted_ask_price: f64,
    pub order_flow_imbalance: f64, // -1.0 to 1.0 (negative = sell pressure)
    pub depth_ratio: f64, // bid_depth / ask_depth
    pub spread: f64, // Current spread
}

/// Fair price calculator with multiple methods
pub struct FairPriceCalculator {
    method: FairPriceMethod,
    price_history: Vec<f64>, // For trend analysis
    max_history: usize,
}

impl FairPriceCalculator {
    pub fn new(method: FairPriceMethod) -> Self {
        Self {
            method,
            price_history: Vec::new(),
            max_history: 1000,
        }
    }
    
    /// Calculate fair price from order book
    pub fn calculate(&mut self, order_book: &OrderBook) -> Option<FairPriceResult> {
        if !order_book.is_valid() {
            warn!("Invalid order book state");
            return None;
        }
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        
        let mid_price = order_book.mid_price()?;
        let spread = order_book.spread()?;
        
        // Calculate metadata first
        let metadata = self.calculate_metadata(order_book, spread);
        
        // Calculate fair price based on selected method
        let (fair_price, confidence) = match &self.method {
            FairPriceMethod::MidPrice => {
                (mid_price, self.calculate_mid_price_confidence(&metadata))
            }
            FairPriceMethod::VolumeWeighted { levels } => {
                self.calculate_volume_weighted(order_book, *levels)
            }
            FairPriceMethod::MicroPrice => {
                self.calculate_micro_price(order_book, &metadata)
            }
        };
        
        // Update price history
        self.update_price_history(fair_price);
        
        let result = FairPriceResult {
            fair_price,
            calculation_method: self.method.to_string(),
            timestamp,
            confidence,
            spread,
            mid_price,
            metadata,
        };
        
        debug!(
            "Fair price calculated: {:.4} (method: {}, confidence: {:.2})",
            result.fair_price, result.calculation_method, result.confidence
        );
        
        Some(result)
    }
    
    /// Calculate volume-weighted average price
    fn calculate_volume_weighted(&self, order_book: &OrderBook, levels: usize) -> (f64, f64) {
        let (top_bids, top_asks) = order_book.get_top_levels(levels);
        
        if top_bids.is_empty() || top_asks.is_empty() {
            return (order_book.mid_price().unwrap_or(0.0), 0.0);
        }
        
        // Calculate volume-weighted bid price
        let (bid_sum, bid_volume) = top_bids.iter().fold((0.0, 0.0), |acc, level| {
            (acc.0 + level.price.0 * level.quantity, acc.1 + level.quantity)
        });
        
        // Calculate volume-weighted ask price
        let (ask_sum, ask_volume) = top_asks.iter().fold((0.0, 0.0), |acc, level| {
            (acc.0 + level.price.0 * level.quantity, acc.1 + level.quantity)
        });
        
        if bid_volume == 0.0 || ask_volume == 0.0 {
            return (order_book.mid_price().unwrap_or(0.0), 0.0);
        }
        
        let weighted_bid = bid_sum / bid_volume;
        let weighted_ask = ask_sum / ask_volume;
        
        // Fair price is volume-weighted average of both sides
        let total_volume = bid_volume + ask_volume;
        let fair_price = (weighted_bid * bid_volume + weighted_ask * ask_volume) / total_volume;
        
        // Confidence based on volume balance
        let volume_balance = (bid_volume - ask_volume).abs() / total_volume;
        let confidence = 1.0 - volume_balance; // Higher confidence when volumes are balanced
        
        (fair_price, confidence.max(0.1))
    }
    
    /// Calculate micro-price (considers order flow imbalance)
    fn calculate_micro_price(&self, order_book: &OrderBook, metadata: &FairPriceMetadata) -> (f64, f64) {
        let best_bid = order_book.best_bid();
        let best_ask = order_book.best_ask();
        
        if best_bid.is_none() || best_ask.is_none() {
            return (order_book.mid_price().unwrap_or(0.0), 0.0);
        }
        
        let bid_price = best_bid.unwrap().price.0;
        let ask_price = best_ask.unwrap().price.0;
        let bid_qty = best_bid.unwrap().quantity;
        let ask_qty = best_ask.unwrap().quantity;
        
        // Micro-price formula: weighted by relative quantities
        let total_qty = bid_qty + ask_qty;
        if total_qty == 0.0 {
            return (order_book.mid_price().unwrap_or(0.0), 0.0);
        }
        
        // Weight towards the side with more liquidity
        let micro_price = (ask_price * bid_qty + bid_price * ask_qty) / total_qty;
        
        // Adjust for order flow imbalance
        let imbalance_adjustment = metadata.order_flow_imbalance * (ask_price - bid_price) * 0.1;
        let adjusted_price = micro_price + imbalance_adjustment;
        
        // Confidence based on liquidity balance and spread tightness
        let qty_balance = 1.0 - (bid_qty - ask_qty).abs() / total_qty;
        let spread_tightness = 1.0 / (1.0 + metadata.spread / order_book.mid_price().unwrap_or(1.0));
        let confidence = (qty_balance * 0.7 + spread_tightness * 0.3).max(0.1);
        
        (adjusted_price, confidence)
    }
    
    /// Calculate metadata for fair price analysis
    fn calculate_metadata(&self, order_book: &OrderBook, spread: f64) -> FairPriceMetadata {
        let (top_bids, top_asks) = order_book.get_top_levels(5);
        
        // Calculate volumes
        let bid_volume: f64 = top_bids.iter().map(|level| level.quantity).sum();
        let ask_volume: f64 = top_asks.iter().map(|level| level.quantity).sum();
        let total_volume = bid_volume + ask_volume;
        
        // Calculate weighted prices
        let weighted_bid_price = if bid_volume > 0.0 {
            top_bids.iter().map(|level| level.price.0 * level.quantity).sum::<f64>() / bid_volume
        } else {
            0.0
        };
        
        let weighted_ask_price = if ask_volume > 0.0 {
            top_asks.iter().map(|level| level.price.0 * level.quantity).sum::<f64>() / ask_volume
        } else {
            0.0
        };
        
        // Order flow imbalance: positive = buy pressure, negative = sell pressure
        let order_flow_imbalance = if total_volume > 0.0 {
            (bid_volume - ask_volume) / total_volume
        } else {
            0.0
        };
        
        // Depth ratio
        let depth_ratio = if ask_volume > 0.0 {
            bid_volume / ask_volume
        } else {
            f64::INFINITY
        };
        
        FairPriceMetadata {
            bid_volume,
            ask_volume,
            total_volume,
            weighted_bid_price,
            weighted_ask_price,
            order_flow_imbalance,
            depth_ratio,
            spread,
        }
    }
    
    /// Calculate confidence for mid-price method
    fn calculate_mid_price_confidence(&self, metadata: &FairPriceMetadata) -> f64 {
        if metadata.total_volume == 0.0 {
            return 0.0;
        }
        
        // Confidence factors:
        // 1. Volume balance (balanced volumes = higher confidence)
        let volume_balance = 1.0 - (metadata.bid_volume - metadata.ask_volume).abs() / metadata.total_volume;
        
        // 2. Total liquidity (more liquidity = higher confidence)
        let liquidity_factor = (metadata.total_volume / (metadata.total_volume + 100.0)).min(1.0);
        
        // 3. Spread tightness (tighter spread = higher confidence)
        let spread_factor = if metadata.spread > 0.0 {
            1.0 / (1.0 + metadata.spread * 1000.0) // Normalize spread impact
        } else {
            0.0
        };
        
        // Weighted combination
        (volume_balance * 0.4 + liquidity_factor * 0.3 + spread_factor * 0.3).max(0.1)
    }
    
    /// Update price history for trend analysis
    fn update_price_history(&mut self, price: f64) {
        self.price_history.push(price);
        if self.price_history.len() > self.max_history {
            self.price_history.remove(0);
        }
    }
    
    /// Get price volatility from recent history
    pub fn get_price_volatility(&self, window: usize) -> Option<f64> {
        if self.price_history.len() < window {
            return None;
        }
        
        let recent_prices: Vec<f64> = self.price_history
            .iter()
            .rev()
            .take(window)
            .cloned()
            .collect();
            
        let mean = recent_prices.iter().sum::<f64>() / recent_prices.len() as f64;
        let variance = recent_prices
            .iter()
            .map(|price| (price - mean).powi(2))
            .sum::<f64>() / recent_prices.len() as f64;
            
        Some(variance.sqrt())
    }
    
    /// Get price trend (positive = upward, negative = downward)
    pub fn get_price_trend(&self, window: usize) -> Option<f64> {
        if self.price_history.len() < window {
            return None;
        }
        
        let recent_prices: Vec<f64> = self.price_history
            .iter()
            .rev()
            .take(window)
            .cloned()
            .collect();
            
        if recent_prices.len() < 2 {
            return None;
        }
        
        // Simple linear trend calculation
        let first_price = recent_prices.last()?;
        let last_price = recent_prices.first()?;
        
        Some((last_price - first_price) / first_price)
    }
    
    /// Update calculation method
    pub fn set_method(&mut self, method: FairPriceMethod) {
        self.method = method;
    }
    
    /// Get current method
    pub fn get_method(&self) -> &FairPriceMethod {
        &self.method
    }
}

impl FairPriceResult {
    /// Get human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Fair Price: ${:.4} | Method: {} | Confidence: {:.1}% | Spread: ${:.4} | Flow: {:.2}",
            self.fair_price,
            self.calculation_method,
            self.confidence * 100.0,
            self.spread,
            self.metadata.order_flow_imbalance
        )
    }
    
    /// Check if result indicates strong buy/sell signal
    pub fn market_signal(&self) -> MarketSignal {
        let imbalance = self.metadata.order_flow_imbalance;
        let confidence_threshold = 0.7;
        
        if self.confidence < confidence_threshold {
            return MarketSignal::Neutral;
        }
        
        if imbalance > 0.3 {
            MarketSignal::BuyPressure
        } else if imbalance < -0.3 {
            MarketSignal::SellPressure
        } else {
            MarketSignal::Balanced
        }
    }
}

/// Market signal based on order flow
#[derive(Debug, Clone, PartialEq)]
pub enum MarketSignal {
    BuyPressure,
    SellPressure,
    Balanced,
    Neutral,
}

impl std::fmt::Display for MarketSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketSignal::BuyPressure => write!(f, "🟢 Buy Pressure"),
            MarketSignal::SellPressure => write!(f, "🔴 Sell Pressure"),
            MarketSignal::Balanced => write!(f, "⚪ Balanced"),
            MarketSignal::Neutral => write!(f, "⚫ Neutral"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order_book::OrderBook;
    
    #[test]
    fn test_fair_price_calculation() {
        let mut calculator = FairPriceCalculator::new(FairPriceMethod::MidPrice);
        let mut order_book = OrderBook::new("BTCUSDT".to_string());
        
        // Add some test data
        order_book.bids.insert(
            crate::order_book::Price::new(50000.0),
            crate::order_book::OrderBookLevel::new(50000.0, 1.0)
        );
        order_book.asks.insert(
            crate::order_book::Price::new(50001.0),
            crate::order_book::OrderBookLevel::new(50001.0, 1.0)
        );
        
        let result = calculator.calculate(&order_book);
        assert!(result.is_some());
        
        let result = result.unwrap();
        assert_eq!(result.fair_price, 50000.5);
        assert!(result.confidence > 0.0);
    }
}