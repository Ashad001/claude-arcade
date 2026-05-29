#!/bin/sh
# Hook: Stop
# Writes done state when Claude finishes its turn.
# Always exits 0 — never blocks Claude Code.

trap 'exit 0' EXIT INT TERM

STATE_DIR="${HOME}/.claude-arcade"
STATE_FILE="${STATE_DIR}/state.json"

mkdir -p "$STATE_DIR" 2>/dev/null || true

UPDATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "")

printf '{"status":"done","updated_at":"%s"}\n' "$UPDATED_AT" > "$STATE_FILE" 2>/dev/null || true

exit 0
