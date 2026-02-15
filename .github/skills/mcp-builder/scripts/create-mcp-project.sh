#!/bin/bash
# create-mcp-project.sh: Scaffold a new MCP server project
# Usage: create-mcp-project.sh --lang rust --name my-mcp-server

set -e

LANG=""
NAME=""

while [[ $# -gt 0 ]]; do
  case $1 in
    --lang)
      LANG="$2"
      shift 2
      ;;
    --name)
      NAME="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1"
      exit 1
      ;;
  esac
done

if [[ -z "$LANG" || -z "$NAME" ]]; then
  echo "Usage: $0 --lang <rust|python|ts> --name <project-name>"
  exit 1
fi

case $LANG in
  rust)
    cargo new "$NAME" --bin
    echo "[MCP] Rust project '$NAME' created."
    ;;
  python)
    mkdir -p "$NAME"
    touch "$NAME/__init__.py"
    echo "[MCP] Python project '$NAME' created."
    ;;
  ts)
    mkdir -p "$NAME/src"
    echo "{}" > "$NAME/package.json"
    echo "[MCP] TypeScript project '$NAME' created."
    ;;
  *)
    echo "Unsupported language: $LANG"
    exit 1
    ;;
esac
