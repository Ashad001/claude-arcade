# claude-arcade

Terminal Minesweeper that runs in a tmux pane while Claude Code works — so you stay in the terminal instead of doom-scrolling Twitter, and actually see it when Claude needs your permission.

```
┌─────────────────────────┬─────────────────────────────┐
│ claude (main pane)      │ claude-arcade (side pane)   │
│                         │                             │
│ > write me a script     │   MINESWEEPER  ●  Score:42  │
│ ⏺ working...            │   ██ ██ 1  .  ██ ██ ██ ██   │
│                         │   ██  1  .  .   1  2 ██ ██  │
│                         │   ⏺ Claude is working: Bash  │
└─────────────────────────┴─────────────────────────────┘
```

When Claude needs a permission prompt, the game **freezes**, the border **flashes red**, and a **bell rings** — impossible to miss without leaving the terminal.

## Install

**macOS / Linux** (requires [tmux](https://github.com/tmux/tmux/wiki/Installing)):

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Ashad001/claude-arcade/releases/latest/download/claude-arcade-installer.sh | sh
```

Then wire up the Claude Code hooks (one-time):

```bash
claude-arcade install
```

### Windows (via WSL2)

Windows users can run claude-arcade by using WSL2 — everything (Claude Code, tmux, and the game) runs inside the Linux environment, so the POSIX hook scripts work normally.

**1. Set up WSL2** (PowerShell, run as admin):

```powershell
wsl --install
```

Restart when prompted, then open a WSL terminal (Ubuntu by default).

**2. Install dependencies inside WSL:**

```bash
sudo apt update && sudo apt install tmux jq
```

**3. Install Claude Code inside WSL:**

```bash
npm install -g @anthropic-ai/claude-code
```

**4. Install claude-arcade inside WSL:**

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/Ashad001/claude-arcade/releases/latest/download/claude-arcade-installer.sh | sh

claude-arcade install
```

**Important:** Run `claude` from the WSL terminal, not from PowerShell or Windows Terminal using a Windows shell. The hooks are registered in the WSL home directory (`~/.claude/`) — a separate config from any Windows-native Claude Code install. Using the wrong terminal means the hooks won't fire.

### Homebrew (macOS)

```bash
brew install Ashad001/tap/claude-arcade
```

### Cargo (Rust developers)

```bash
cargo install claude-arcade
```

## Requirements

- **tmux** — the game opens in a split pane
- **Claude Code** — hooks fire on tool calls / notifications
- macOS or Linux (Windows users: see [WSL2 instructions](#windows-via-wsl2) above)

Install tmux if missing:

```bash
# macOS
brew install tmux

# Debian/Ubuntu
sudo apt install tmux

# Fedora
sudo dnf install tmux

# Arch
sudo pacman -S tmux
```

## How it works

Every Claude Code tool call fires a shell hook that writes a small JSON file to `~/.claude-arcade/state.json`. The game polls that file every 100ms and reacts visually:

| Claude status | Border | Footer message |
|---|---|---|
| Working | Blue | `⏺ Claude is working: Bash` |
| Needs permission | **Red flashing** + 🔔 bell | `⚠ CLAUDE NEEDS PERMISSION — SWITCH PANES` |
| Idle / waiting | Yellow | `⏸ Claude is waiting for your input` |
| Done | Green (3s) | `✓ Claude finished` |
| No session | Plain | Key hint bar |

The **permission state freezes the game** and pauses your score multiplier — missing a permission prompt has a cost.

## Controls

| Key | Action |
|---|---|
| Arrow keys / `hjkl` | Move cursor |
| `Space` / `Enter` | Reveal cell |
| `f` | Toggle flag |
| `r` | Restart board |
| `Tab` | Toggle leaderboard overlay |
| `q` / `Esc` | Quit |

## Difficulty

```bash
claude-arcade play --difficulty easy    # 9×9,  10 mines
claude-arcade play --difficulty medium  # 16×16, 40 mines  (default)
claude-arcade play --difficulty hard    # 30×16, 99 mines
```

## Local development & testing

```bash
# 1. Build
cargo build --release

# 2. Play standalone (no Claude integration)
cargo run -- play

# 3. Simulate hook events manually
echo '{"status":"working","tool":"Bash","updated_at":"2026-05-22T14:00:00Z"}' \
  > ~/.claude-arcade/state.json

echo '{"status":"permission_needed","updated_at":"2026-05-22T14:00:01Z"}' \
  > ~/.claude-arcade/state.json
# → red flashing border, terminal bell, game freezes

echo '{"status":"done","updated_at":"2026-05-22T14:00:02Z"}' \
  > ~/.claude-arcade/state.json
# → green border for 3 seconds

# 4. Install hooks (dry-run)
cargo run -- install --dry-run

# 5. Run tests
cargo test
```

## Uninstall

```bash
claude-arcade uninstall
```

Removes the Claude Code hooks and the `~/.claude/hooks/claude-arcade/` directory. Your `~/.claude-arcade/stats.json` score history is kept.

## Privacy

No telemetry. No network calls. Ever. The only outbound request in the entire stack is the one-time binary download at install time. The game only reads a local file that the hooks write.

## How to release (maintainers)

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions builds binaries for all four targets and publishes the release automatically. See `.github/workflows/release.yml`.

## License

MIT
