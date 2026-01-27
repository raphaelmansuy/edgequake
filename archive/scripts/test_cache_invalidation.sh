#!/usr/bin/env bash
# Manual test script for Dashboard stats cache invalidation fix
#
# This script helps verify that the cache invalidation works correctly

set -e

echo "=== Dashboard Stats Cache Invalidation Manual Test ==="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

function check_backend() {
  echo "Checking backend health..."
  if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Backend is running${NC}"
    return 0
  else
    echo -e "${RED}✗ Backend is not running${NC}"
    echo "Run: make backend-bg"
    return 1
  fi
}

function check_frontend() {
  echo "Checking frontend..."
  if curl -s http://localhost:3000 > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Frontend is running${NC}"
    return 0
  else
    echo -e "${RED}✗ Frontend is not running${NC}"
    echo "Run: cd edgequake_webui && npm run dev"
    return 1
  fi
}

function show_workspace_stats() {
  local workspace_id=$1
  echo ""
  echo "Fetching stats for workspace: $workspace_id"
  curl -s "http://localhost:8080/api/v1/workspaces/$workspace_id/stats" | jq '.'
}

function main() {
  # Check services are running
  check_backend || exit 1
  check_frontend || exit 1

  echo ""
  echo "=== Manual Testing Steps ==="
  echo ""
  echo "1. Open browser to: http://localhost:3000"
  echo "2. Open Developer Tools (F12)"
  echo "3. Open Console tab"
  echo "4. Look for these log messages:"
  echo "   - [Dashboard] Render:"
  echo "   - [Dashboard] Cache validation complete"
  echo "   - [Dashboard] Workspace changed, forcing stats refetch"
  echo ""
  echo "5. Check Network tab for these API calls:"
  echo "   - GET /api/v1/workspaces/{id}/stats"
  echo "   - Should see fresh call on every page load"
  echo ""
  echo "6. Check Application > Local Storage:"
  echo "   - edgequake-cache-version should exist"
  echo "   - version should be 'v1.0.0'"
  echo "   - tenantId and workspaceId should match current selection"
  echo ""
  echo "=== Test Scenario: Stale Cache Detection ==="
  echo ""
  echo "7. In Console, run this to simulate stale cache:"
  echo "   localStorage.setItem('edgequake-cache-version', JSON.stringify({"
  echo "     tenantId: 'old-id',"
  echo "     workspaceId: 'old-id',"
  echo "     version: 'v0.9.0',"
  echo "     timestamp: Date.now() - 3600000"
  echo "   }))"
  echo ""
  echo "8. Reload the page (F5)"
  echo "9. Check Console for:"
  echo "   - [CacheManager] Version mismatch: v0.9.0 → v1.0.0"
  echo "   - [CacheManager] Cache is stale, clearing all caches"
  echo "   - [CacheManager] Clearing all React Query caches"
  echo ""
  echo "10. Verify stats show correct values (not 0)"
  echo ""
  echo "=== Test Scenario: Workspace Change ==="
  echo ""
  echo "11. Switch workspaces using workspace selector"
  echo "12. Check Console for:"
  echo "    - [Dashboard] Workspace changed, forcing stats refetch: {new-id}"
  echo ""
  echo "13. Check Network tab for new /stats API call"
  echo "14. Verify stats update to show new workspace data"
  echo ""
  echo "=== Expected Results ==="
  echo ""
  echo -e "${GREEN}✓${NC} Stats API called on every page load"
  echo -e "${GREEN}✓${NC} Stale cache detected and cleared automatically"
  echo -e "${GREEN}✓${NC} Stats update when workspace changes"
  echo -e "${GREEN}✓${NC} No more 0/0 stats from stale cache"
  echo ""
  
  # Show example workspace stats
  echo "=== Example: Fetching stats for a workspace ==="
  echo ""
  echo "To get workspace ID, run:"
  echo "  curl -s http://localhost:8080/api/v1/workspaces | jq '.items[] | {id, name}'"
  echo ""
  
  # Get first workspace and show stats
  local workspace_id=$(curl -s http://localhost:8080/api/v1/workspaces | jq -r '.items[0].id // empty')
  if [ -n "$workspace_id" ]; then
    show_workspace_stats "$workspace_id"
  else
    echo "No workspaces found. Create one first."
  fi
}

main "$@"
