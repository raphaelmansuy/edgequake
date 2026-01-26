#!/bin/bash
#
# Test script to verify dashboard stats fix
#
# This script:
# 1. Starts the dev services in background
# 2. Waits for services to be ready
# 3. Checks that backend API returns correct stats for WorkspaceA
# 4. Opens browser to dashboard to manually verify
#

set -e

echo "🧪 Testing Dashboard Stats Fix"
echo "=============================="

# Stop any existing services
echo "→ Stopping existing services..."
cd /Users/raphaelmansuy/Github/03-working/edgequake
make stop 2>/dev/null || true

# Start services in background
echo "→ Starting services in background..."
make dev-bg

# Wait for services to start
echo "→ Waiting for services to start (20 seconds)..."
sleep 20

# Test backend API
echo "→ Testing backend API..."
WORKSPACE_ID="23d89fe3-e822-4c06-8f8c-82752436f7f3"

echo "  Fetching stats for WorkspaceA..."
STATS=$(curl -s "http://localhost:8080/api/v1/workspaces/${WORKSPACE_ID}/stats")
echo "  Response: $STATS"

ENTITY_COUNT=$(echo $STATS | jq -r '.entity_count')
REL_COUNT=$(echo $STATS | jq -r '.relationship_count')

echo ""
echo "📊 Backend API Test Results:"
echo "  Entities: $ENTITY_COUNT (expected: 8)"
echo "  Relationships: $REL_COUNT (expected: 6)"

if [ "$ENTITY_COUNT" == "8" ] && [ "$REL_COUNT" == "6" ]; then
  echo "  ✅ Backend API returns correct data"
else
  echo "  ❌ Backend API data mismatch!"
  exit 1
fi

# Open browser to dashboard
echo ""
echo "→ Opening browser to test frontend..."
echo "  URL: http://localhost:3000/"
echo ""
echo "📝 Manual Test Steps:"
echo "  1. Dashboard should auto-select WorkspaceA"
echo "  2. URL should update to /?workspace=workspacea"
echo "  3. Stats should show:"
echo "     - Documents: 1"
echo "     - Entities: 8"
echo "     - Relationships: 6"
echo "     - Entity Types: 1"
echo ""

open "http://localhost:3000/"

echo "Press Ctrl+C when done testing..."
wait
