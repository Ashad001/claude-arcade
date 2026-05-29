#!/bin/sh
# Hook: SessionStart
# Launches claude-arcade in a tmux split pane when a Claude Code session begins.
# Always exits 0 — never blocks Claude Code.

# Belt-and-suspenders: any uncaught error exits cleanly
trap 'exit 0' EXIT INT TERM

# Skip on WSL — cross-environment binary execution is unreliable
# (the hook fires fine; the binary launch is the part that breaks)
if [ -f /proc/version ] && grep -qi 'microsoft\|wsl' /proc/version 2>/dev/null; then
    exit 0
fi

# Must be inside a tmux session
[ -n "${TMUX:-}" ] || exit 0

# tmux must be available
command -v tmux >/dev/null 2>&1 || exit 0

# Find a native (non-Windows) claude-arcade binary
BINARY=""
for candidate in \
    "$HOME/.local/bin/claude-arcade" \
    "/usr/local/bin/claude-arcade" \
    "$(command -v claude-arcade 2>/dev/null || true)"
do
    # Skip Windows PE binaries that WSL exposes via interop
    case "$candidate" in
        *.exe | /mnt/c/* | /mnt/d/* | /mnt/e/*) continue ;;
    esac
    if [ -x "$candidate" ]; then
        BINARY="$candidate"
        break
    fi
done

[ -n "$BINARY" ] || exit 0

# Launch in a right-split pane (35% width), best-effort
tmux split-window -h -p 35 "$BINARY play" 2>/dev/null || true

exit 0
