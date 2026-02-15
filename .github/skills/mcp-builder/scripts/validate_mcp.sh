#!/bin/bash
# validate_mcp.sh: Validate MCP project structure (wrapper for Python script)

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <project-path>"
  exit 1
fi

python3 "$(dirname "$0")/validate_mcp.py" --project "$1"
