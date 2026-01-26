#!/bin/bash
# Test script for workspace-tenant auto-validation feature
#
# This script simulates the bug scenario and verifies auto-correction:
# 1. Show current tenant/workspace context
# 2. Verify workspace stats are correct
# 3. Check that workspace belongs to the displayed tenant

set -e

API_BASE="http://localhost:8080/api/v1"
FRONTEND="http://localhost:3000"

echo "=========================================="
echo "Workspace-Tenant Auto-Validation Test"
echo "=========================================="
echo ""

# Get all tenants
echo "📊 Step 1: List all tenants"
TENANTS=$(curl -s "$API_BASE/tenants" | jq -r '.items[] | "\(.id) - \(.name)"')
echo "$TENANTS"
echo ""

# Get workspaces for each tenant
echo "📊 Step 2: List workspaces per tenant"
for tenant_line in $(curl -s "$API_BASE/tenants" | jq -r '.items[] | "\(.id)|\(.name)"'); do
  tenant_id=$(echo "$tenant_line" | cut -d'|' -f1)
  tenant_name=$(echo "$tenant_line" | cut -d'|' -f2)
  
  echo "Tenant: $tenant_name ($tenant_id)"
  
  workspaces=$(curl -s "$API_BASE/tenants/$tenant_id/workspaces" | jq -r '.items[] | "  - \(.name) (\(.id))"')
  if [ -z "$workspaces" ]; then
    echo "  (no workspaces)"
  else
    echo "$workspaces"
  fi
  echo ""
done

# Check for workspace ID 676b8da6 (the one with 13 entities)
echo "📊 Step 3: Verify target workspace (676b8da6)"
TARGET_WS="676b8da6-d203-4530-89a5-8c9100c78b47"

if workspace_info=$(curl -s "$API_BASE/workspaces/$TARGET_WS" 2>/dev/null); then
  tenant_id=$(echo "$workspace_info" | jq -r '.tenant_id')
  workspace_name=$(echo "$workspace_info" | jq -r '.name')
  
  echo "✓ Workspace found: $workspace_name"
  echo "  ID: $TARGET_WS"
  echo "  Tenant ID: $tenant_id"
  
  # Get tenant name
  tenant_name=$(curl -s "$API_BASE/tenants/$tenant_id" | jq -r '.name')
  echo "  Tenant Name: $tenant_name"
  
  # Get stats
  stats=$(curl -s "$API_BASE/workspaces/$TARGET_WS/stats")
  entity_count=$(echo "$stats" | jq -r '.entity_count')
  relationship_count=$(echo "$stats" | jq -r '.relationship_count')
  
  echo "  Stats: $entity_count entities, $relationship_count relationships"
  echo ""
  
  if [ "$entity_count" == "13" ]; then
    echo "✅ SUCCESS: This is the correct workspace with 13 entities"
  else
    echo "⚠️  WARNING: Expected 13 entities, got $entity_count"
  fi
else
  echo "❌ ERROR: Workspace $TARGET_WS not found"
  exit 1
fi

echo ""
echo "=========================================="
echo "Frontend Auto-Validation Test Instructions"
echo "=========================================="
echo ""
echo "To test the auto-validation feature:"
echo ""
echo "1. Open browser DevTools (F12)"
echo "2. Go to Application tab → Local Storage → http://localhost:3000"
echo "3. Look for key: 'edgequake-tenant-store'"
echo "4. Note the current values:"
echo "   - selectedTenantId"
echo "   - selectedWorkspaceId"
echo ""
echo "5. Open Dashboard: $FRONTEND"
echo "6. Check browser Console for auto-validation logs:"
echo "   [WorkspaceTenantValidator] Should show 'Valid' or 'Auto-correcting'"
echo ""
echo "7. Verify UI shows: '$tenant_name / $workspace_name'"
echo "8. Verify Dashboard shows: $entity_count entities, $relationship_count relationships"
echo ""
echo "To test auto-correction with corrupted data:"
echo ""
echo "9. Edit localStorage 'edgequake-tenant-store' → 'state' → 'selectedWorkspaceId'"
echo "10. Change it to: '00000003-0000-4000-a000-000000000003'"
echo "    (This is 'Default Workspace' from Default tenant with 0 entities)"
echo "11. Keep selectedTenantId as: '$tenant_id' (TennantZZ)"
echo "12. Refresh page"
echo "13. Console should show: '[WorkspaceTenantValidator] Auto-correcting to workspace: ...'"
echo "14. Dashboard should automatically show correct stats (13 entities)"
echo ""
echo "✅ Test complete!"
