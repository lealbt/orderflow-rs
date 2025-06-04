#!/bin/bash

# OrderFlow-RS - Development Runner Script
# This script helps with building, testing, and running the bot

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
SYMBOL="BTCUSDT"
METHOD="mid-price"
LOG_LEVEL="info"
BUILD_TYPE="debug"

# Function to display usage
usage() {
    echo -e "${BLUE}OrderFlow-RS - Development Runner${NC}"
    echo ""
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  build     Build the project"
    echo "  test      Run tests"
    echo "  run       Run the bot"
    echo "  clean     Clean build artifacts"
    echo "  docker    Build and run Docker container"
    echo "  check     Run cargo check and clippy"
    echo ""
    echo "Options:"
    echo "  --symbol SYMBOL      Trading symbol (default: BTCUSDT)"
    echo "  --method METHOD      Calculation method (default: mid-price)"
    echo "  --log-level LEVEL    Log level (default: info)"
    echo "  --release            Use release build"
    echo "  --help               Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 run --symbol ETHUSDT --method volume-weighted"
    echo "  $0 build --release"
    echo "  $0 test"
    echo "  $0 docker"
}

# Function to print colored messages
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if Rust is installed
check_rust() {
    if ! command -v cargo &> /dev/null; then
        print_error "Rust/Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    print_status "Rust version: $(rustc --version)"
}

# Function to build the project
build_project() {
    print_status "Building project..."
    
    if [ "$BUILD_TYPE" = "release" ]; then
        print_status "Building in release mode..."
        cargo build --release
        print_success "Release build completed"
    else
        print_status "Building in debug mode..."
        cargo build
        print_success "Debug build completed"
    fi
}

# Function to run tests
run_tests() {
    print_status "Running tests..."
    cargo test
    print_success "All tests passed"
}

# Function to run the bot
run_bot() {
    print_status "Starting OrderFlow-RS..."
    print_status "Symbol: $SYMBOL"
    print_status "Method: $METHOD"
    print_status "Log Level: $LOG_LEVEL"
    
    if [ "$BUILD_TYPE" = "release" ]; then
        cargo run --release -- --symbol "$SYMBOL" --method "$METHOD" --log-level "$LOG_LEVEL"
    else
        cargo run -- --symbol "$SYMBOL" --method "$METHOD" --log-level "$LOG_LEVEL"
    fi
}

# Function to clean artifacts
clean_project() {
    print_status "Cleaning build artifacts..."
    cargo clean
    print_success "Clean completed"
}

# Function to run code checks
check_code() {
    print_status "Running cargo check..."
    cargo check
    
    print_status "Running clippy..."
    cargo clippy -- -D warnings
    
    print_status "Checking formatting..."
    cargo fmt --check
    
    print_success "All checks passed"
}

# Function to build and run Docker container
docker_run() {
    print_status "Building Docker image..."
    docker build -t orderflow-rs .
    
    print_status "Running Docker container..."
    docker run --rm -it orderflow-rs --symbol "$SYMBOL" --method "$METHOD" --log-level "$LOG_LEVEL"
}

# Parse command line arguments
COMMAND=""
while [[ $# -gt 0 ]]; do
    case $1 in
        build|test|run|clean|docker|check)
            COMMAND="$1"
            shift
            ;;
        --symbol)
            SYMBOL="$2"
            shift 2
            ;;
        --method)
            METHOD="$2"
            shift 2
            ;;
        --log-level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        --release)
            BUILD_TYPE="release"
            shift
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Check if command was provided
if [ -z "$COMMAND" ]; then
    print_error "No command provided"
    usage
    exit 1
fi

# Validate method
case $METHOD in
    "mid-price"|"volume-weighted"|"micro-price")
        ;;
    *)
        print_error "Invalid method: $METHOD"
        print_error "Valid methods: mid-price, volume-weighted, micro-price"
        exit 1
        ;;
esac

# Validate log level
case $LOG_LEVEL in
    "trace"|"debug"|"info"|"warn"|"error")
        ;;
    *)
        print_error "Invalid log level: $LOG_LEVEL"
        print_error "Valid levels: trace, debug, info, warn, error"
        exit 1
        ;;
esac

# Main execution
print_status "OrderFlow-RS - Development Runner"
print_status "Command: $COMMAND"

case $COMMAND in
    build)
        check_rust
        build_project
        ;;
    test)
        check_rust
        run_tests
        ;;
    run)
        check_rust
        build_project
        run_bot
        ;;
    clean)
        clean_project
        ;;
    check)
        check_rust
        check_code
        ;;
    docker)
        if ! command -v docker &> /dev/null; then
            print_error "Docker not found. Please install Docker."
            exit 1
        fi
        docker_run
        ;;
    *)
        print_error "Unknown command: $COMMAND"
        usage
        exit 1
        ;;
esac

print_success "Script completed successfully"