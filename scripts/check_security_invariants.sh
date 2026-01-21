#!/bin/bash
# Security Invariant Checker
# OODA-230: This script enforces critical security invariants
#
# Usage: ./scripts/check_security_invariants.sh
# Exit code: 0 if all checks pass, 1 if any fail

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EDGEQUAKE_DIR="$REPO_ROOT/edgequake/crates"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

FAILED=0

echo "========================================"
echo "Security Invariant Checker (OODA-230)"
echo "========================================"
echo ""

# ------------------------------------------------------------------------------
# Invariant 1: SAFE_PROVIDER_CREATION
# All production provider creation must use safe variants
# ------------------------------------------------------------------------------
echo -n "Checking SAFE_PROVIDER_CREATION... "

# Exclude: test files, mod.rs (documentation), resolver.rs (internal method name)
UNSAFE_PATTERNS=$(grep -rn --include="*.rs" \
  -e 'ProviderFactory::create_llm_provider(' \
  -e 'ProviderFactory::create_embedding_provider(' \
  "$EDGEQUAKE_DIR/edgequake-api/src/" 2>/dev/null \
  | grep -v '/tests/' \
  | grep -v 'mod.rs:' \
  | grep -v 'resolver.rs:.*fn create_llm_provider' \
  | grep -v '#\[doc' \
  | grep -v '//!' \
  | grep -v '/// ' \
  || true)

if [ -n "$UNSAFE_PATTERNS" ]; then
  echo -e "${RED}FAILED${NC}"
  echo "Found unsafe provider creation in production code:"
  echo "$UNSAFE_PATTERNS"
  echo ""
  echo "FIX: Use ProviderFactory::create_safe_*_provider() instead"
  FAILED=1
else
  echo -e "${GREEN}PASSED${NC}"
fi

# ------------------------------------------------------------------------------
# Invariant 2: TENANT_ISOLATION
# Query operations must use workspace.tenant_id, not header tenant_id
# Check for suspicious patterns where header tenant_id is used for data queries
# ------------------------------------------------------------------------------
echo -n "Checking TENANT_ISOLATION... "

# Look for patterns that might indicate using header tenant_id for data queries
# SAFE patterns:
#   - data_tenant_id (derived from workspace)
#   - workspace.*tenant_id
#   - // OODA-231 comment
# UNSAFE patterns:
#   - with_tenant_id(tenant_ctx.tenant_id) directly
TENANT_ISSUES=$(grep -rn --include="*.rs" \
  -e 'with_tenant_id.*tenant_ctx\.tenant_id' \
  "$EDGEQUAKE_DIR/edgequake-api/src/handlers/" 2>/dev/null \
  | grep -v 'data_tenant_id' \
  | grep -v '// OODA-231' \
  | head -5 \
  || true)

# This check is informational only for now
if [ -n "$TENANT_ISSUES" ]; then
  echo -e "${YELLOW}WARNING${NC}"
  echo "Potential tenant isolation issues (manual review needed):"
  echo "$TENANT_ISSUES"
else
  echo -e "${GREEN}PASSED${NC}"
fi

# ------------------------------------------------------------------------------
# Invariant 3: NO_UNWRAP_IN_HANDLERS
# Production handlers should not use .unwrap() - use proper error handling
# ------------------------------------------------------------------------------
echo -n "Checking NO_UNWRAP_IN_HANDLERS... "

UNWRAP_COUNT=$(grep -rn --include="*.rs" \
  -e '\.unwrap()' \
  "$EDGEQUAKE_DIR/edgequake-api/src/handlers/" 2>/dev/null \
  | grep -v '/tests/' \
  | grep -v 'test_' \
  | wc -l \
  || echo "0")

if [ "$UNWRAP_COUNT" -gt 10 ]; then
  echo -e "${YELLOW}WARNING${NC} ($UNWRAP_COUNT instances)"
  echo "Consider using proper error handling instead of .unwrap()"
else
  echo -e "${GREEN}PASSED${NC} ($UNWRAP_COUNT instances, threshold: 10)"
fi

# ------------------------------------------------------------------------------
# Invariant 4: PROVIDER_MODULE_EXISTS
# The unified provider resolver module must exist
# ------------------------------------------------------------------------------
echo -n "Checking PROVIDER_MODULE_EXISTS... "

PROVIDER_FILES=(
  "$EDGEQUAKE_DIR/edgequake-api/src/providers/mod.rs"
  "$EDGEQUAKE_DIR/edgequake-api/src/providers/resolver.rs"
  "$EDGEQUAKE_DIR/edgequake-api/src/providers/error.rs"
)

MISSING_FILES=""
for file in "${PROVIDER_FILES[@]}"; do
  if [ ! -f "$file" ]; then
    MISSING_FILES="$MISSING_FILES $file"
  fi
done

if [ -n "$MISSING_FILES" ]; then
  echo -e "${RED}FAILED${NC}"
  echo "Missing provider module files:$MISSING_FILES"
  FAILED=1
else
  echo -e "${GREEN}PASSED${NC}"
fi

# ------------------------------------------------------------------------------
# Invariant 5: PATH_VALIDATION (OODA-248)
# Path validation module must exist and be used in scan_directory
# ------------------------------------------------------------------------------
echo -n "Checking PATH_VALIDATION... "

PATH_VALIDATION_FILE="$EDGEQUAKE_DIR/edgequake-api/src/path_validation.rs"
if [ ! -f "$PATH_VALIDATION_FILE" ]; then
  echo -e "${RED}FAILED${NC}"
  echo "Missing path_validation.rs module"
  FAILED=1
else
  # Check that scan_directory uses path validation
  SCAN_USES_VALIDATION=$(grep -c 'validate_path' \
    "$EDGEQUAKE_DIR/edgequake-api/src/handlers/documents.rs" 2>/dev/null || echo "0")
  
  if [ "$SCAN_USES_VALIDATION" -eq 0 ]; then
    echo -e "${RED}FAILED${NC}"
    echo "scan_directory handler does not use validate_path"
    FAILED=1
  else
    echo -e "${GREEN}PASSED${NC}"
  fi
fi

# ------------------------------------------------------------------------------
# Summary
# ------------------------------------------------------------------------------
echo ""
echo "========================================"
if [ $FAILED -eq 0 ]; then
  echo -e "${GREEN}All security invariants passed!${NC}"
  exit 0
else
  echo -e "${RED}Security invariant check failed!${NC}"
  echo "Please fix the issues above before committing."
  exit 1
fi
