use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeStatus {
    Working,
    PermissionNeeded,
    Idle,
    Done,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ClaudeState {
    #[serde(default)]
    pub status: ClaudeStatus,
    pub tool: Option<String>,
    pub message: Option<String>,
    pub updated_at: Option<String>,
}

pub fn state_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude-arcade").join("state.json"))
}

pub fn read_state() -> ClaudeState {
    let Some(path) = state_file_path() else {
        return ClaudeState::default();
    };
    let Ok(contents) = fs::read_to_string(&path) else {
        return ClaudeState::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}
