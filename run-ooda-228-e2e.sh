#!/bin/bash

# OODA-228: Complete E2E Testing Setup & Execution
# This script starts all services and runs Playwright tests in headed mode

set -e

PROJECT_DIR="/Users/raphaelmansuy/Github/03-working/edgequake"
BACKEND_DIR="$PROJECT_DIR/edgequake"
FRONTEND_DIR="$PROJECT_DIR/edgequake_webui"

BACKEND_PORT=8080
FRONTEND_PORT=3001

echo "🚀 OODA-228 E2E Test Setup"
echo "================================"
echo ""

# Function to check if service is running
check_service() {
  local port=$1
  local name=$2
  
  if curl -s http://localhost:$port/ > /dev/null 2>&1 || curl -s http://localhost:$port/health > /dev/null 2>&1; then
    echo "✅ $name is running on port $port"
    return 0
  else
    echo "❌ $name is NOT running on port $port"
    return 1
  fi
}

# Start backend if not running
echo "📡 Starting Backend..."
if ! check_service $BACKEND_PORT "Backend"; then
  echo "   Starting backend in background..."
  cd "$BACKEND_DIR"
  
  # Build and run in background
  cargo build --release 2>/dev/null &
  CARGO_PID=$!
  
  # Wait for build to complete
  echo "   Waiting for build to complete..."
  wait $CARGO_PID
  
  # Run the built binary
  timeout 300 cargo run --release 2>&1 | grep -E "INFO|Starting|Server|Error" &
  BACKEND_PID=$!
  
  echo "   Backend PID: $BACKEND_PID"
  echo "   Waiting for backend to start..."
  
  # Wait for backend to be ready
  for i in {1..30}; do
    if curl -s http://localhost:$BACKEND_PORT/health > /dev/null 2>&1; then
      echo "✅ Backend is ready"
      break
    fi
    echo "   Waiting... ($i/30)"
    sleep 1
  done
else
  BACKEND_PID=$(pgrep -f "edgequake" || echo "unknown")
fi

echo ""

# Start frontend if not running
echo "🌐 Starting Frontend..."
if ! check_service $FRONTEND_PORT "Frontend"; then
  echo "   Starting dev server..."
  cd "$FRONTEND_DIR"
  
  # Install dependencies if needed
  if [ ! -d "node_modules" ]; then
    echo "   Installing dependencies..."
    npm install 2>&1 | tail -3
  fi
  
  # Start dev server in background
  npm run dev -- --port $FRONTEND_PORT 2>&1 | grep -E "Local:|ready" &
  FRONTEND_PID=$!
  
  echo "   Frontend PID: $FRONTEND_PID"
  echo "   Waiting for frontend to start..."
  
  # Wait for frontend to be ready
  for i in {1..30}; do
    if curl -s http://localhost:$FRONTEND_PORT/ > /dev/null 2>&1; then
      echo "✅ Frontend is ready"
      break
    fi
    echo "   Waiting... ($i/30)"
    sleep 1
  done
else
  FRONTEND_PID=$(pgrep -f "npm run dev" || echo "unknown")
fi

echo ""
echo "✅ Services are ready!"
echo ""
echo "🧪 Running Playwright Tests..."
echo "   - Tests will show in browser (headed mode)"
echo "   - You can see interactions in real-time"
echo "   - Check console output for diagnostics"
echo ""

# Run tests
cd "$FRONTEND_DIR"

npx playwright test e2e/ooda-228-workspace-embedding.spec.ts \
  --headed \
  --reporter=list,html

echo ""
echo "✅ Tests completed!"
echo ""
echo "📊 Test Report:"
echo "   Open: file://$FRONTEND_DIR/playwright-report/index.html"
echo ""
