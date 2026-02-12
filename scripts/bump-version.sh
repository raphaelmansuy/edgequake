#!/bin/bash
# bump-version.sh: Bump version in VERSION, Cargo.toml, and package.json, and create a git tag.
# Usage: ./scripts/bump-version.sh <new_version>

set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <new_version>"
  exit 1
fi

NEW_VERSION="$1"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# Update root VERSION file
echo "$NEW_VERSION" > "$ROOT_DIR/VERSION"

# Update all Cargo.toml files
find "$ROOT_DIR/edgequake" -name Cargo.toml | while read -r file; do
  sed -i '' -E "s/^version ?= ?\"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$NEW_VERSION\"/" "$file"
done

# Update frontend package.json
jq ".version = \"$NEW_VERSION\"" "$ROOT_DIR/edgequake_webui/package.json" > "$ROOT_DIR/edgequake_webui/package.json.tmp" && mv "$ROOT_DIR/edgequake_webui/package.json.tmp" "$ROOT_DIR/edgequake_webui/package.json"

# Optionally update CHANGELOG.md (manual step recommended)

echo "Version bumped to $NEW_VERSION."
echo "Don't forget to update CHANGELOG.md and commit your changes."

git add VERSION edgequake/**/Cargo.toml edgequake_webui/package.json

echo "Run: git commit -m 'Bump version to $NEW_VERSION' && git tag v$NEW_VERSION && git push && git push --tags"
