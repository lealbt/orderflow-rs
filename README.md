# OrderFlow-RS 🚀

A high-performance Rust application that connects to Binance WebSocket API to calculate real-time fair prices from order book data using multiple sophisticated algorithms.

## 🌟 Features

- **Real-time Order Book Processing**: Live WebSocket connection to Binance
- **Multiple Fair Price Algorithms**:
  - Mid-Price: Simple bid-ask average
  - Volume-Weighted: VWAP across top N levels
  - Micro-Price: Advanced algorithm considering order flow imbalance
- **Robust Architecture**: Async/await with proper error handling
- **Production-Ready**: Configurable logging, health checks, and reconnection logic
- **State-of-the-Art**: Modern Rust patterns with thread-safe concurrent processing

## 🔧 Technical Highlights

- **Async WebSocket Management**: Tokio-based async runtime
- **Thread-Safe Order Book**: RwLock-protected concurrent data structures  
- **Smart Reconnection**: Exponential backoff with configurable retry limits
- **Memory Efficient**: BTreeMap for ordered price levels with depth limiting
- **Type Safety**: Strong typing with ordered floats for price precision

## 📦 Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/orderflow-rs
cd orderflow-rs

# Build the project
cargo build --release

# Run with default settings (BTCUSDT, mid-price method)
cargo run --release

# Run with custom symbol and method
cargo run --release -- --symbol ETHUSDT --method volume-weighted --log-level debug
```

## 🚀 Usage

### Basic Usage
```bash
# Default: BTCUSDT with mid-price calculation
./target/release/orderflow-rs

# Custom symbol
./target/release/orderflow-rs --symbol ETHUSDT

# Different calculation methods
./target/release/orderflow-rs --method volume-weighted
./target/release/orderflow-rs --method micro-price
```

### Command Line Options
```
Options:
  -s, --symbol <SYMBOL>      Trading symbol [default: BTCUSDT]
  -l, --log-level <LEVEL>    Log level [default: info]
  -m, --method <METHOD>      Fair price calculation method [default: mid-price]
  -h, --help                 Print help information
  -V, --version              Print version information
```

## 📊 Fair Price Calculation Methods

### 1. Mid-Price
Simple average of best bid and ask prices:
```
Fair Price = (Best Bid + Best Ask) / 2
```

### 2. Volume-Weighted Average Price (VWAP)
Considers volume at multiple price levels:
```
Fair Price = Σ(Price × Volume) / Σ(Volume)
```

### 3. Micro-Price
Advanced algorithm that accounts for order flow imbalance:
```
Micro Price = (Ask × Bid Quantity + Bid × Ask Quantity) / (Bid Quantity + Ask Quantity)
```
Plus imbalance adjustment based on market pressure.

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│   WebSocket     │────│  Order Book      │────│  Fair Price         │
│   Manager       │    │  Manager         │    │  Calculator         │
└─────────────────┘    └──────────────────┘    └─────────────────────┘
         │                       │                        │
         │                       │                        │
    ┌─────────┐            ┌─────────────┐         ┌─────────────┐
    │ Binance │            │ Thread-Safe │         │  Multiple   │
    │   API   │            │ Data Store  │         │ Algorithms  │
    └─────────┘            └─────────────┘         └─────────────┘
```

### Core Components

- **WebSocketManager**: Handles connection lifecycle and message processing
- **OrderBookManager**: Thread-safe order book state management
- **FairPriceCalculator**: Multiple algorithmic approaches for price calculation
- **BinanceClient**: REST API integration for symbol validation and snapshots

## 🔧 Configuration

The application uses a flexible configuration system:

```rust
// Default configuration
Config {
    symbol: "BTCUSDT",
    calculation_method: MidPrice,
    websocket: {
        reconnect_attempts: 5,
        reconnect_delay_ms: 1000,
        ping_interval_ms: 30000,
    },
    order_book: {
        max_depth: 100,
        update_threshold_us: 1000,
    }
}
```

## 📈 Sample Output

```
┌─ BTCUSDT Fair Price Update ─────────────────────────────────┐
│ Fair Price: $43,247.8500    Method: Volume-Weighted         │
│ Mid Price:  $43,247.7500    Confidence: 87.3%               │
│ Best Bid:   $43,247.2500    Best Ask: $43,248.2500         │
│ Spread:     $1.0000         (0.002%)                        │
│ Signal:     🟢 Buy Pressure                                 │
│ Volumes:    Bid: 15.42      Ask: 12.78      Total: 28.20   │
│ Flow:       0.32                                            │
└─────────────────────────────────────────────────────────────┘
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Test specific module
cargo test order_book

# Integration tests
cargo test --test integration
```

## 📋 Dependencies

### Core Dependencies
- **tokio**: Async runtime
- **tokio-tungstenite**: WebSocket client
- **reqwest**: HTTP client for REST API
- **serde/serde_json**: Serialization
- **anyhow/thiserror**: Error handling

### Data Structures
- **dashmap**: Concurrent HashMap
- **ordered-float**: Ordered floating point numbers
- **futures-util**: Async utilities

### CLI & Logging
- **clap**: Command line argument parsing
- **tracing**: Structured logging
- **tracing-subscriber**: Log formatting

## 🚀 Performance Optimizations

1. **Zero-Copy Message Processing**: Direct deserialization from WebSocket messages
2. **Efficient Order Book Updates**: BTreeMap for O(log n) price level operations
3. **Memory Pool**: Reused objects to minimize allocations
4. **Concurrent Processing**: Lock-free where possible, RwLock for shared state
5. **Batch Updates**: Process multiple order book changes in single transaction

## 🔒 Error Handling

Comprehensive error handling strategy:

- **Connection Failures**: Automatic reconnection with exponential backoff
- **Message Parsing**: Graceful handling of malformed data
- **API Rate Limits**: Built-in respect for Binance API limits
- **Network Issues**: Timeout handling and health checks

## 📊 Monitoring & Metrics

Optional Prometheus metrics (enable with `--features metrics`):

- Connection uptime
- Message processing rate
- Fair price calculation latency
- Order book update frequency
- WebSocket reconnection count

## 🔮 Future Enhancements

- [ ] Multiple symbol support
- [ ] Historical data analysis
- [ ] Machine learning price prediction
- [ ] REST API server mode
- [ ] Database persistence
- [ ] Advanced order flow analytics
- [ ] Arbitrage opportunity detection

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## ⚠️ Disclaimer

This software is for educational and research purposes only. It is not financial advice and should not be used for actual trading without proper risk management and understanding of the markets.

## 🔗 Links

- [Binance API Documentation](https://binance-docs.github.io/apidocs/)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
- [WebSocket Protocol RFC](https://tools.ietf.org/html/rfc6455)

## 👨‍💻 Author

Built with ❤️ by [Your Name] - showcasing modern Rust development practices for high-frequency financial applications.

---

*This project demonstrates production-ready Rust code suitable for quantitative finance and high-performance trading systems.*