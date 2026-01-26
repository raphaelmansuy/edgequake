#!/bin/bash

# OODA-228: Complete E2E Testing Setup & Execution
# This script starts all services and runs Playwright tests in headed mode

set -e

PROJECT_DIR="/Users/raphaelmansuy/Github/03-working/edgequake"
BACKEND_DIR="$PROJECT_DIR/edgequake"
FRONTEND_DIR="$PROJECT_DIR/edgequake_webui"

BACKEND_PORT=8080
FRONTEND_PORT=3001

echo "80 OODA-228 E2E Test Setup"
echo "================================"
echo ""

# Function to check if service is running
check_service() {
  local port=$1
  local name=$2
  
  if curl -s http://localhost:$port/ > /dev/null 2>&1 || curl -s http://localhost:$port/health > /dev/null 2>&1; then
    echo "05 $name is running on port $port"
    return 0
  else
    echo "4c $name is NOT running on port $port"
    return 1
  fi
}

# Start backend if not running
echo "e1 Starting Backend..."
if ! check_service $BACKEND_PORT "Backend"; then
  echo "   Starting backend in background..."
  cd "$BACKEND_DIR"
  
  # Build and run in background
  cargo build --release 2>/dev/null &
