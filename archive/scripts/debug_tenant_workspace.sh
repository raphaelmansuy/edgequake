#!/bin/bash

echo "🔍 EdgeQuake Tenant/Workspace Debug Script"
echo "=========================================="
echo ""

# Tenant badc48ee with workspace 676b8da6 (13 entities/9 relationships)
echo "📊 Tenant: TennantZZ (badc48ee)"
echo "   Workspaces:"
curl -s "http://localhost:8080/api/v1/tenants/badc48ee-331a-4e0a-b40d-56de0fb7ceaa/workspaces" | jq -r '.items[] | "   - \(.name) (\(.id)) - Stats: \(.entity_count // "N/A") entities"' || echo "   ERROR fetching workspaces"

echo ""
echo "   Workspace 676b8da6 Stats:"
curl -s "http://localhost:8080/api/v1/workspaces/676b8da6-d203-4530-89a5-8c9100c78b47/stats" | jq '{entity_count, relationship_count, document_count}'

echo ""
echo "=========================================="
echo ""

# Default tenant with workspace 00000003 and 23d89fe3
echo "📊 Tenant: Default (00000002)"
echo "   Workspaces:"
curl -s "http://localhost:8080/api/v1/tenants/00000000-0000-0000-0000-000000000002/workspaces" | jq -r '.items[] | "   - \(.name) (\(.id[:8])) - Stats: \(.entity_count // "N/A") entities"' || echo "   ERROR fetching workspaces"

echo ""
echo "   Default Workspace (00000003) Stats:"
curl -s "http://localhost:8080/api/v1/workspaces/00000000-0000-0000-0000-000000000003/stats" | jq '{entity_count, relationship_count, document_count}'

echo ""
echo "   WorkspaceA (23d89fe3) Stats:"
curl -s "http://localhost:8080/api/v1/workspaces/23d89fe3-e822-4c06-8f8c-82752436f7f3/stats" | jq '{entity_count, relationship_count, document_count}'

echo ""
echo "=========================================="
echo ""
echo "💡 CONCLUSION:"
echo "   - Dashboard showing 0 entities/0 relationships"
echo "   - Workspace page showing 13 entities/9 relationships"
echo "   - Problem: Different tenant context between pages!"
echo ""
echo "   Dashboard is using tenant 00000002 (Default)"
echo "   Workspace is using tenant badc48ee (TennantZZ)"
echo ""
