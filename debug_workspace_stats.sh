#!/usr/bin/env bash
# Debug script to find which workspace has which stats

echo "=== Debugging Dashboard Stats Issue ==="
echo ""

# Get all tenants
echo "1. Fetching all tenants..."
TENANTS=$(curl -s "http://localhost:8080/api/v1/tenants")
echo "$TENANTS" | jq '.'
echo ""

# Extract first tenant ID
TENANT_ID=$(echo "$TENANTS" | jq -r '.items[0].id // empty')

if [ -z "$TENANT_ID" ]; then
  echo "ERROR: No tenants found"
  exit 1
fi

echo "Using tenant ID: $TENANT_ID"
echo ""

# Get all workspaces for this tenant
echo "2. Fetching all workspaces for tenant $TENANT_ID..."
WORKSPACES=$(curl -s "http://localhost:8080/api/v1/tenants/$TENANT_ID/workspaces")
echo "$WORKSPACES" | jq '.'
echo ""

# Get stats for each workspace
echo "3. Fetching stats for each workspace..."
echo ""

echo "$WORKSPACES" | jq -r '.items[] | .id' | while read -r WORKSPACE_ID; do
  WORKSPACE_NAME=$(echo "$WORKSPACES" | jq -r ".items[] | select(.id == \"$WORKSPACE_ID\") | .name")
  WORKSPACE_SLUG=$(echo "$WORKSPACES" | jq -r ".items[] | select(.id == \"$WORKSPACE_ID\") | .slug")
  
  echo "----------------------------------------"
  echo "Workspace: $WORKSPACE_NAME (slug: $WORKSPACE_SLUG)"
  echo "ID: $WORKSPACE_ID"
  echo ""
  
  STATS=$(curl -s "http://localhost:8080/api/v1/workspaces/$WORKSPACE_ID/stats" \
    -H "X-Tenant-ID: $TENANT_ID" \
    -H "X-Workspace-ID: $WORKSPACE_ID")
  
  echo "Stats:"
  echo "$STATS" | jq '.'
  echo ""
  
  # Extract key stats
  DOC_COUNT=$(echo "$STATS" | jq -r '.document_count // 0')
  ENTITY_COUNT=$(echo "$STATS" | jq -r '.entity_count // 0')
  REL_COUNT=$(echo "$STATS" | jq -r '.relationship_count // 0')
  
  echo "Summary: $DOC_COUNT docs, $ENTITY_COUNT entities, $REL_COUNT relationships"
  echo ""
done

echo "========================================" echo ""
echo "🔍 ANALYSIS:"
echo "- Check which workspace has 13 entities and 9 relationships"
echo "- Compare with workspace shown in Dashboard UI"
echo "- If mismatch, the wrong workspace ID is being used"
echo ""
