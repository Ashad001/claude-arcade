use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    /// "minesweeper" | "tictactoe" | "2048"
    #[serde(default = "default_game")]
    pub game: String,
    pub difficulty: String,
    pub score: u32,
    pub time_secs: u64,
    pub won: bool,
    pub timestamp: String, // "YYYY-MM-DDTHH:MM:SSZ"
    // minesweeper-specific (zero for other games)
    #[serde(default)]
    pub board_width: usize,
    #[serde(default)]
    pub board_height: usize,
    #[serde(default)]
    pub mine_count: usize,
}

fn default_game() -> String {
    "minesweeper".into()
}

pub fn stats_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude-arcade").join("stats.json"))
}

pub fn current_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_rfc3339(secs)
}

/// Hand-rolled RFC3339 formatter — avoids a chrono dependency.
fn format_rfc3339(unix_secs: u64) -> String {
    let sec = (unix_secs % 60) as u32;
    let min = ((unix_secs / 60) % 60) as u32;
    let hour = ((unix_secs / 3600) % 24) as u32;
    let days = unix_secs / 86400;

    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

/// Read all saved records. Returns an empty vec on any error — never panics.
pub fn read_stats() -> Vec<GameRecord> {
    let Some(path) = stats_file_path() else {
        return vec![];
    };
    let Ok(contents) = fs::read_to_string(&path) else {
        return vec![];
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

/// Append a record to the stats file.
pub fn append_record(record: GameRecord) -> std::io::Result<()> {
    let Some(path) = stats_file_path() else {
        return Ok(());
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut records = read_stats();
    records.push(record);

    if records.len() > 100 {
        let drop = records.len() - 100;
        records.drain(..drop);
    }

    let json = serde_json::to_string_pretty(&records).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Return the top `n` records sorted: wins first, then score desc, then time asc.
pub fn leaderboard_top(n: usize) -> Vec<GameRecord> {
    let mut records = read_stats();
    records.sort_by(|a, b| {
        b.won
            .cmp(&a.won)
            .then(b.score.cmp(&a.score))
            .then(a.time_secs.cmp(&b.time_secs))
    });
    records.truncate(n);
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_epoch() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn rfc3339_known_date() {
        assert_eq!(format_rfc3339(1_779_460_200), "2026-05-22T14:30:00Z");
    }

    #[test]
    fn leaderboard_sort_order() {
        let records = vec![
            GameRecord {
                game: "minesweeper".into(),
                difficulty: "easy".into(),
                score: 100,
                time_secs: 60,
                won: false,
                timestamp: "".into(),
                board_width: 9,
                board_height: 9,
                mine_count: 10,
            },
            GameRecord {
                game: "minesweeper".into(),
                difficulty: "medium".into(),
                score: 200,
                time_secs: 120,
                won: true,
                timestamp: "".into(),
                board_width: 16,
                board_height: 16,
                mine_count: 40,
            },
            GameRecord {
                game: "minesweeper".into(),
                difficulty: "medium".into(),
                score: 300,
                time_secs: 90,
                won: true,
                timestamp: "".into(),
                board_width: 16,
                board_height: 16,
                mine_count: 40,
            },
        ];

        let mut sorted = records.clone();
        sorted.sort_by(|a, b| {
            b.won
                .cmp(&a.won)
                .then(b.score.cmp(&a.score))
                .then(a.time_secs.cmp(&b.time_secs))
        });

        assert!(sorted[0].won);
        assert_eq!(sorted[0].score, 300);
        assert!(sorted[1].won);
        assert_eq!(sorted[1].score, 200);
        assert!(!sorted[2].won);
    }
}
