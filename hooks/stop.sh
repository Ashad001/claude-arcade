#!/bin/sh
# Hook: Stop
# Writes done state when Claude finishes its turn.
# Must always exit 0.
set -eu

STATE_DIR="$HOME/.claude-arcade"
STATE_FILE="$STATE_DIR/state.json"

mkdir -p "$STATE_DIR"

UPDATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "")

printf '{"status":"done","updated_at":"%s"}\n' "$UPDATED_AT" > "$STATE_FILE"

exit 0
