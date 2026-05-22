use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRecord {
    pub difficulty: String, // "easy" | "medium" | "hard"
    pub score: u32,
    pub time_secs: u64,
    pub won: bool,
    pub timestamp: String, // "YYYY-MM-DDTHH:MM:SSZ"
    pub board_width: usize,
    pub board_height: usize,
    pub mine_count: usize,
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
    let days = unix_secs / 86400; // days since 1970-01-01

    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

/// Convert a count of days since the Unix epoch to (year, month, day).
/// Uses the standard proleptic Gregorian algorithm.
fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
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
/// Creates the file if it doesn't exist. Caps history at 100 entries (oldest dropped).
/// Errors are returned but should be ignored by the caller so the game never crashes.
pub fn append_record(record: GameRecord) -> std::io::Result<()> {
    let Some(path) = stats_file_path() else {
        return Ok(());
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut records = read_stats();
    records.push(record);

    // Keep the most recent 100 entries
    if records.len() > 100 {
        let drop = records.len() - 100;
        records.drain(..drop);
    }

    let json = serde_json::to_string_pretty(&records)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Atomic write: temp file → rename
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
        // 2026-05-22T14:30:00Z = 1,779,460,200 seconds
        assert_eq!(format_rfc3339(1_779_460_200), "2026-05-22T14:30:00Z");
    }

    #[test]
    fn leaderboard_sort_order() {
        let records = vec![
            GameRecord {
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

        // Inject into a file-free sort test
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
