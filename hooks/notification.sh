#!/bin/sh
# Hook: Notification
# Routes to permission_needed or idle based on message content.
# Must always exit 0.
set -eu

STATE_DIR="$HOME/.claude-arcade"
STATE_FILE="$STATE_DIR/state.json"

mkdir -p "$STATE_DIR"

INPUT=$(cat)
MESSAGE=$(printf '%s' "$INPUT" | jq -r '.message // ""' 2>/dev/null || echo "")
UPDATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "")

# Claude Code sends permission prompts with recognisable message content
if printf '%s' "$MESSAGE" | grep -qi "permission\|allow\|approve"; then
    STATUS="permission_needed"
else
    STATUS="idle"
fi

# Escape message for JSON (strip quotes/backslashes to keep it safe)
SAFE_MSG=$(printf '%s' "$MESSAGE" | tr -d '"\\')

printf '{"status":"%s","message":"%s","updated_at":"%s"}\n' \
    "$STATUS" "$SAFE_MSG" "$UPDATED_AT" > "$STATE_FILE"

exit 0
