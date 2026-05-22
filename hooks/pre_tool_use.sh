#!/bin/sh
# Hook: PreToolUse
# Writes working state to ~/.claude-arcade/state.json before each tool call.
# Must be fast (<50ms) and must always exit 0.
set -eu

STATE_DIR="$HOME/.claude-arcade"
STATE_FILE="$STATE_DIR/state.json"

mkdir -p "$STATE_DIR"

# Read tool name from hook input (stdin JSON)
INPUT=$(cat)
TOOL_NAME=$(printf '%s' "$INPUT" | jq -r '.tool_name // "unknown"' 2>/dev/null || echo "unknown")
UPDATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "")

printf '{"status":"working","tool":"%s","updated_at":"%s"}\n' \
    "$TOOL_NAME" "$UPDATED_AT" > "$STATE_FILE"

exit 0
