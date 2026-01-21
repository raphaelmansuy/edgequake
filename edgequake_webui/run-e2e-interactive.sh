#!/bin/bash

# OODA-228 Interactive E2E Test Script
# This script sets up and runs Playwright tests in headed/interactive mode

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BACKEND_PORT=8080
FRONTEND_PORT=3001
BACKEND_URL="http://localhost:$BACKEND_PORT"
FRONTEND_URL="http://localhost:$FRONTEND_PORT"

echo "🚀 OODA-228 Interactive E2E Test Runner"
echo "======================================="
echo ""

# Check if backend is running
echo "📡 Checking backend status..."
if ! curl -s "$BACKEND_URL/health" > /dev/null 2>&1; then
  echo "⚠️  Backend not running at $BACKEND_URL"
  echo "   (This is OK - tests can run against a starting backend)"
else
  echo "✅ Backend is running at $BACKEND_URL"
fi

# Check if frontend dev server is running
echo ""
echo "🌐 Checking frontend status..."
if ! curl -s "$FRONTEND_URL/" > /dev/null 2>&1; then
  echo "⚠️  Frontend dev server not running at $FRONTEND_URL"
  echo "   Playwright will start it automatically (or use PLAYWRIGHT_BASE_URL)"
else
  echo "✅ Frontend is running at $FRONTEND_URL"
fi

echo ""
echo "📝 About to run interactive tests..."
echo "   - Tests will open a browser window (headed mode)"
echo "   - You can interact with the application"
echo "   - Press Ctrl+C to stop any time"
echo ""

# Run Playwright tests in headed mode
# Headed mode shows the browser window
# --debug opens the playwright inspector for interactive debugging
cd "$PROJECT_ROOT"

echo "🧪 Starting Playwright tests in headed mode..."
echo ""

# Run the OODA-228 test
npx playwright test e2e/ooda-228-workspace-embedding.spec.ts --headed

echo ""
echo "✅ Test run completed!"
echo ""
echo "📊 Test Report:"
echo "   HTML Report: $PROJECT_ROOT/playwright-report/index.html"
echo ""
