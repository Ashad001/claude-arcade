use color_eyre::eyre::{bail, eyre, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write as IoWrite};
use std::path::{Path, PathBuf};

const HOOKS_SUBDIR: &str = "claude-arcade";

pub fn install(yes: bool, dry_run: bool) -> Result<()> {
    check_tmux()?;

    let hooks_dir = claude_hooks_dir()?;
    let settings_path = claude_settings_path()?;

    println!("claude-arcade install");
    println!();

    // Build list of changes
    let hook_scripts = hook_script_contents();
    let mut changes: Vec<String> = Vec::new();

    for (name, _) in &hook_scripts {
        let dest = hooks_dir.join(name);
        if dest.exists() {
            changes.push(format!("  overwrite  {}", dest.display()));
        } else {
            changes.push(format!("  create     {}", dest.display()));
        }
    }

    let existing_settings = read_settings(&settings_path)?;
    let merged = merge_hooks(existing_settings, &hooks_dir)?;
    changes.push(format!("  update     {}", settings_path.display()));

    println!("Changes to apply:");
    for change in &changes {
        println!("{}", change);
    }
    println!();

    if dry_run {
        println!("(dry-run — no files written)");
        return Ok(());
    }

    if !yes && !confirm("Apply these changes? [Y/n] ")? {
        println!("Aborted.");
        return Ok(());
    }

    // Create directories
    fs::create_dir_all(&hooks_dir)
        .with_context(|| format!("creating {}", hooks_dir.display()))?;

    if let Some(state_dir) = dirs::home_dir().map(|h| h.join(".claude-arcade")) {
        fs::create_dir_all(&state_dir)
            .with_context(|| format!("creating {}", state_dir.display()))?;
    }

    // Write hook scripts
    for (name, content) in &hook_scripts {
        let dest = hooks_dir.join(name);
        write_executable(&dest, content)
            .with_context(|| format!("writing {}", dest.display()))?;
        println!("  wrote  {}", dest.display());
    }

    // Write settings atomically
    write_settings_atomic(&settings_path, &merged)?;
    println!("  updated  {}", settings_path.display());

    println!();
    println!("Done! Start a new Claude Code session to launch the game.");
    println!("Or test manually: claude-arcade play");
    Ok(())
}

pub fn uninstall(yes: bool) -> Result<()> {
    let hooks_dir = claude_hooks_dir()?;
    let settings_path = claude_settings_path()?;

    println!("claude-arcade uninstall");
    println!();

    if !yes && !confirm("Remove claude-arcade hooks? [Y/n] ")? {
        println!("Aborted.");
        return Ok(());
    }

    // Remove hooks directory
    if hooks_dir.exists() {
        fs::remove_dir_all(&hooks_dir)
            .with_context(|| format!("removing {}", hooks_dir.display()))?;
        println!("  removed  {}", hooks_dir.display());
    }

    // Remove hook entries from settings.json
    let settings = read_settings(&settings_path)?;
    let cleaned = remove_hooks(settings, &hooks_dir);
    write_settings_atomic(&settings_path, &cleaned)?;
    println!("  updated  {}", settings_path.display());
    println!();
    println!("Uninstalled. Your ~/.claude-arcade/stats.json is preserved.");
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn check_tmux() -> Result<()> {
    if which_tmux() {
        return Ok(());
    }
    bail!(
        "✗ claude-arcade requires tmux, which was not found in PATH.\n\
         \n\
         Install with:\n\
         \n\
           macOS:   brew install tmux\n\
           Debian:  sudo apt install tmux\n\
           Fedora:  sudo dnf install tmux\n\
           Arch:    sudo pacman -S tmux\n\
         \n\
         Then re-run: claude-arcade install"
    );
}

fn which_tmux() -> bool {
    std::process::Command::new("tmux")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn claude_hooks_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| eyre!("cannot determine home directory"))?;
    Ok(home.join(".claude").join("hooks").join(HOOKS_SUBDIR))
}

fn claude_settings_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| eyre!("cannot determine home directory"))?;
    Ok(home.join(".claude").join("settings.json"))
}

fn read_settings(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let contents = fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("parsing {}", path.display()))
}

fn merge_hooks(mut settings: Value, hooks_dir: &Path) -> Result<Value> {
    let hooks_obj = settings
        .as_object_mut()
        .ok_or_else(|| eyre!("settings.json is not a JSON object"))?
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| eyre!("settings.hooks is not a JSON object"))?;

    let event_map = hook_event_map(hooks_dir);

    for (event, command) in event_map {
        let entry = hooks_obj.entry(&event).or_insert_with(|| json!([]));
        let arr = entry
            .as_array_mut()
            .ok_or_else(|| eyre!("hooks.{} is not an array", event))?;

        let hook_obj = json!({
            "matcher": "",
            "hooks": [{ "type": "command", "command": command }]
        });

        // Deduplicate: remove any existing entry pointing to our command
        arr.retain(|v| {
            v.get("hooks")
                .and_then(|h| h.as_array())
                .map(|hs| {
                    !hs.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map(|c| c.contains("claude-arcade"))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(true)
        });

        arr.push(hook_obj);
    }

    Ok(settings)
}

fn remove_hooks(mut settings: Value, hooks_dir: &Path) -> Value {
    if let Some(hooks_obj) = settings
        .get_mut("hooks")
        .and_then(|h| h.as_object_mut())
    {
        let prefix = hooks_dir.to_string_lossy().to_string();
        for arr in hooks_obj.values_mut() {
            if let Some(entries) = arr.as_array_mut() {
                entries.retain(|v| {
                    v.get("hooks")
                        .and_then(|h| h.as_array())
                        .map(|hs| {
                            !hs.iter().any(|h| {
                                h.get("command")
                                    .and_then(|c| c.as_str())
                                    .map(|c| c.contains(&prefix) || c.contains("claude-arcade"))
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(true)
                });
            }
        }
    }
    settings
}

fn write_settings_atomic(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let pretty = serde_json::to_string_pretty(value)?;
    fs::write(&tmp, pretty)?;
    fs::rename(&tmp, path).with_context(|| format!("renaming to {}", path.display()))?;
    Ok(())
}

fn write_executable(path: &Path, content: &str) -> Result<()> {
    fs::write(path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn confirm(prompt: &str) -> Result<bool> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    let answer = buf.trim().to_lowercase();
    Ok(answer.is_empty() || answer == "y" || answer == "yes")
}

fn hook_event_map(hooks_dir: &Path) -> Vec<(String, String)> {
    let dir = hooks_dir.to_string_lossy();
    vec![
        ("SessionStart".into(), format!("{dir}/session_start.sh")),
        ("PreToolUse".into(), format!("{dir}/pre_tool_use.sh")),
        ("Notification".into(), format!("{dir}/notification.sh")),
        ("Stop".into(), format!("{dir}/stop.sh")),
        ("SessionEnd".into(), format!("{dir}/session_end.sh")),
    ]
}

fn hook_script_contents() -> Vec<(String, String)> {
    vec![
        ("session_start.sh".into(), include_str!("../hooks/session_start.sh").into()),
        ("pre_tool_use.sh".into(), include_str!("../hooks/pre_tool_use.sh").into()),
        ("notification.sh".into(), include_str!("../hooks/notification.sh").into()),
        ("stop.sh".into(), include_str!("../hooks/stop.sh").into()),
        ("session_end.sh".into(), include_str!("../hooks/session_end.sh").into()),
    ]
}
