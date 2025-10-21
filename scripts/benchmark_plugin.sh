#!/bin/bash

# Nylon Plugin Performance Benchmark Script
# This script measures the performance improvements from optimizations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$PROJECT_ROOT/benchmark_results"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Nylon Plugin Performance Benchmark                    ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Create results directory
mkdir -p "$RESULTS_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RESULT_FILE="$RESULTS_DIR/benchmark_$TIMESTAMP.txt"

echo "Results will be saved to: $RESULT_FILE"
echo ""

# Function to print section header
print_header() {
    echo -e "\n${YELLOW}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${YELLOW}  $1${NC}"
    echo -e "${YELLOW}═══════════════════════════════════════════════════════════${NC}\n"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# 1. Build Nylon
print_header "1. Building Nylon (Release)"
cd "$PROJECT_ROOT"
cargo build --release
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Nylon built successfully${NC}"
else
    echo -e "${RED}✗ Failed to build Nylon${NC}"
    exit 1
fi

# 2. Build Go Plugin
print_header "2. Building Go Plugin"
cd "$PROJECT_ROOT/examples/go"
go build -buildmode=c-shared -o plugin_sdk.so main.go
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Go plugin built successfully${NC}"
else
    echo -e "${RED}✗ Failed to build Go plugin${NC}"
    exit 1
fi

# 3. Start Nylon Server
print_header "3. Starting Nylon Server"
cd "$PROJECT_ROOT"
"$PROJECT_ROOT/target/release/nylon" --config "$PROJECT_ROOT/examples/config.yaml" > "$RESULTS_DIR/nylon.log" 2>&1 &
NYLON_PID=$!
echo "Nylon PID: $NYLON_PID"

# Wait for server to start
sleep 3

# Check if server is running
if ! kill -0 $NYLON_PID 2>/dev/null; then
    echo -e "${RED}✗ Nylon server failed to start${NC}"
    cat "$RESULTS_DIR/nylon.log"
    exit 1
fi
echo -e "${GREEN}✓ Nylon server started${NC}"

# Function to cleanup
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    if [ ! -z "$NYLON_PID" ]; then
        kill $NYLON_PID 2>/dev/null || true
    fi
}

trap cleanup EXIT

# 4. Run Benchmarks
print_header "4. Running Benchmarks"

{
    echo "Nylon Plugin Performance Benchmark"
    echo "Timestamp: $(date)"
    echo "======================================"
    echo ""
} > "$RESULT_FILE"

# Test 1: Simple HTTP Request (with plugin)
print_header "Test 1: HTTP Request with Plugin Processing"
if command_exists wrk; then
    echo "Using wrk for HTTP benchmarking..."
    wrk -t4 -c100 -d30s --latency http://localhost:8080/myapp 2>&1 | tee -a "$RESULT_FILE"
elif command_exists hey; then
    echo "Using hey for HTTP benchmarking..."
    hey -z 30s -c 100 -m GET http://localhost:8080/myapp 2>&1 | tee -a "$RESULT_FILE"
else
    echo -e "${YELLOW}⚠ Neither wrk nor hey found. Install with:${NC}"
    echo "  brew install wrk  # macOS"
    echo "  go install github.com/rakyll/hey@latest  # Go"
    echo ""
    echo "Running simple curl test instead..."
    for i in {1..100}; do
        curl -s http://localhost:8080/myapp > /dev/null
    done
    echo "Completed 100 requests"
fi

# Test 2: Request with Headers
print_header "Test 2: Request with Custom Headers"
if command_exists hey; then
    hey -z 10s -c 50 -H "X-Custom-Header: test" http://localhost:8080/authz 2>&1 | tee -a "$RESULT_FILE"
fi

# Test 3: Streaming Response
print_header "Test 3: Streaming Response"
if command_exists hey; then
    hey -z 10s -c 20 http://localhost:8080/stream 2>&1 | tee -a "$RESULT_FILE"
fi

# Test 4: WebSocket (if websocat is available)
print_header "Test 4: WebSocket Performance"
if command_exists websocat; then
    echo "Testing WebSocket connections..."
    
    # Function to test single WebSocket
    test_ws() {
        echo "test message" | websocat ws://localhost:8080/ws -t 1 2>/dev/null
    }
    
    echo "Running 100 WebSocket connections..."
    START=$(date +%s)
    for i in {1..100}; do
        test_ws &
    done
    wait
    END=$(date +%s)
    DURATION=$((END - START))
    echo "Completed 100 WebSocket connections in ${DURATION}s" | tee -a "$RESULT_FILE"
else
    echo -e "${YELLOW}⚠ websocat not found. Install with:${NC}"
    echo "  brew install websocat  # macOS"
    echo "  cargo install websocat  # Cargo"
fi

# Test 5: Memory Usage
print_header "Test 5: Memory Usage"
echo "Memory usage during benchmark:" | tee -a "$RESULT_FILE"
if [ "$(uname)" == "Darwin" ]; then
    # macOS
    ps -o rss,vsz -p $NYLON_PID | tail -n 1 | awk '{print "RSS: " $1 " KB, VSZ: " $2 " KB"}' | tee -a "$RESULT_FILE"
else
    # Linux
    ps -o rss,vsz -p $NYLON_PID | tail -n 1 | awk '{print "RSS: " $1 " KB, VSZ: " $2 " KB"}' | tee -a "$RESULT_FILE"
fi

# Test 6: Latency Distribution
print_header "Test 6: Latency Distribution"
if command_exists wrk; then
    echo "Measuring latency percentiles..." | tee -a "$RESULT_FILE"
    wrk -t4 -c100 -d10s --latency http://localhost:8080/myapp 2>&1 | grep -A 10 "Latency Distribution" | tee -a "$RESULT_FILE"
fi

# Summary
print_header "Benchmark Complete"
echo -e "${GREEN}✓ All tests completed${NC}"
echo ""
echo "Results saved to: $RESULT_FILE"
echo ""

# Display summary
echo -e "${BLUE}Summary:${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Extract key metrics if available
if grep -q "Requests/sec" "$RESULT_FILE"; then
    echo -e "${GREEN}HTTP Performance:${NC}"
    grep "Requests/sec" "$RESULT_FILE" | head -1
    grep "Transfer/sec" "$RESULT_FILE" | head -1 || true
fi

if grep -q "Average:" "$RESULT_FILE"; then
    echo -e "${GREEN}Latency:${NC}"
    grep "Average:" "$RESULT_FILE" | head -1
fi

echo ""
echo -e "${YELLOW}📊 View full results: cat $RESULT_FILE${NC}"
echo -e "${YELLOW}📈 Compare with previous runs in: $RESULTS_DIR/${NC}"
echo ""

# Check server logs for errors
if grep -i "error\|panic\|fatal" "$RESULTS_DIR/nylon.log" >/dev/null; then
    echo -e "${RED}⚠️  Errors detected in server logs:${NC}"
    grep -i "error\|panic\|fatal" "$RESULTS_DIR/nylon.log" | tail -5
else
    echo -e "${GREEN}✓ No errors in server logs${NC}"
fi

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  Benchmark completed successfully!                        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

