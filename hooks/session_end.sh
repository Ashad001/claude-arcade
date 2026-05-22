#!/bin/sh
# Hook: SessionEnd
# Writes done state and kills the claude-arcade tmux pane.
# Must always exit 0.
set -eu

STATE_DIR="$HOME/.claude-arcade"
STATE_FILE="$STATE_DIR/state.json"

mkdir -p "$STATE_DIR"

UPDATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo "")
printf '{"status":"done","updated_at":"%s"}\n' "$UPDATED_AT" > "$STATE_FILE"

# Give the game a moment to render the done banner before killing the pane
sleep 3

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
