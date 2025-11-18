#!/bin/bash
# Sync skills from source tree to ~/.awsdash/skills/
# Usage: ./scripts/sync-skills.sh

set -e

SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/skills"
TARGET_DIR="$HOME/.awsdash/skills"

echo "🔄 Syncing skills from $SOURCE_DIR to $TARGET_DIR"

# Create target directory if it doesn't exist
mkdir -p "$TARGET_DIR"

# Sync skills
rsync -av --delete "$SOURCE_DIR/" "$TARGET_DIR/"

echo "✅ Skills synced successfully"
echo "📊 Skills in $TARGET_DIR:"
ls -1 "$TARGET_DIR"
