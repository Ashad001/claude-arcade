#!/bin/sh
# Hook: SessionEnd
# Writes done state and kills the claude-arcade tmux pane.
# Always exits 0 — never blocks Claude Code.

trap 'exit 0' EXIT INT TERM

STATE_DIR="${HOME}/.claude-arcade"
STATE_FILE="${STATE_DIR}/state.json"

mkdir -p "$STATE_DIR" 2>/dev/null || true

UPDATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "")
printf '{"status":"done","updated_at":"%s"}\n' "$UPDATED_AT" > "$STATE_FILE" 2>/dev/null || true

# Skip tmux teardown on WSL — same cross-environment reliability concern as SessionStart
if [ -f /proc/version ] && grep -qi 'microsoft\|wsl' /proc/version 2>/dev/null; then
    exit 0
fi

# Give the game a moment to render the done banner before killing the pane
sleep 3 2>/dev/null || true

# Kill panes running claude-arcade (best-effort, tmux may not be available)
if [ -n "${TMUX:-}" ] && command -v tmux >/dev/null 2>&1; then
    tmux list-panes -a -F "#{pane_id} #{pane_current_command}" 2>/dev/null \
        | grep -i "claude-arcade" \
        | awk '{print $1}' \
        | while IFS= read -r pane_id; do
            tmux kill-pane -t "$pane_id" 2>/dev/null || true
          done
fi

exit 0
