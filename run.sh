#!/bin/bash

# ONVIF Proxy Startup Script

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}ONVIF Proxy for Reolink Cameras${NC}"
echo "================================"
echo ""

# Check if config file exists
if [ ! -f "config/cameras.yaml" ]; then
    echo -e "${RED}Error: config/cameras.yaml not found${NC}"
    echo ""
    echo "Please create the configuration file:"
    echo "  cp config/cameras.yaml.example config/cameras.yaml"
    echo "  nano config/cameras.yaml"
    echo ""
    exit 1
fi

# Check if binary exists
if [ ! -f "target/release/onvif-proxy" ]; then
    echo -e "${YELLOW}Binary not found. Building...${NC}"
    echo ""
    cargo build --release
    echo ""
fi

# Set default environment variables
export CONFIG_PATH="${CONFIG_PATH:-config/cameras.yaml}"
export RUST_LOG="${RUST_LOG:-info}"

# Detect external IP for base URL
if [ -z "$BASE_URL" ]; then
    EXTERNAL_IP=$(hostname -I | awk '{print $1}')
    export BASE_URL="http://${EXTERNAL_IP}:8000"
fi

echo -e "${GREEN}Configuration:${NC}"
echo "  Config file: $CONFIG_PATH"
echo "  Base URL: $BASE_URL"
echo "  Log level: $RUST_LOG"
echo ""

echo -e "${GREEN}Starting proxy...${NC}"
echo ""

# Run the proxy
exec ./target/release/onvif-proxy
