#!/bin/sh
# Hook: SessionStart
# Launches claude-arcade in a tmux split pane when a Claude Code session begins.
set -eu

# Must be inside a tmux session
[ -n "${TMUX:-}" ] || exit 0

# tmux must be available
command -v tmux >/dev/null 2>&1 || exit 0

# Find the installed binary
BINARY=""
for candidate in \
    "$HOME/.local/bin/claude-arcade" \
    "/usr/local/bin/claude-arcade" \
    "$(command -v claude-arcade 2>/dev/null || true)"
do
    if [ -x "$candidate" ]; then
        BINARY="$candidate"
        break
    fi
done

[ -n "$BINARY" ] || exit 0

# Launch in a right-split pane (35% width), best-effort
tmux split-window -h -p 35 "$BINARY play" 2>/dev/null || true

exit 0
