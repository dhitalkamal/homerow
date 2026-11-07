// progress file: ~/.local/share/homerow/progress.json (or $XDG_DATA_HOME/homerow)
use crate::stats::KeyStats;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

// bump when the shape changes in a way old files can't be read as
pub const VERSION: u32 = 1;
const MAX_ROUNDS: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Round {
    pub mode: String,
    pub wpm: u32,
    pub acc: u32,
    pub at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub version: u32,
    pub stats: KeyStats,
    pub rounds: Vec<Round>,
    pub boxes: BTreeMap<String, u32>,
    // practice ms per utc day, keyed YYYY-MM-DD
    pub days: BTreeMap<String, u64>,
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            version: VERSION,
            stats: KeyStats::new(),
            rounds: Vec::new(),
            boxes: BTreeMap::new(),
            days: BTreeMap::new(),
        }
    }
}

impl Progress {
    pub fn add_round(&mut self, round: Round, practice_ms: u64) {
        *self.days.entry(day_key(round.at_ms)).or_insert(0) += practice_ms;
        self.rounds.push(round);
        if self.rounds.len() > MAX_ROUNDS {
            self.rounds.drain(..self.rounds.len() - MAX_ROUNDS);
        }
    }
}

pub fn default_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        });
    base.join("homerow/progress.json")
}

// a missing file is a fresh start. a corrupt one is moved aside, never deleted,
// and the second value says where it went so the app can tell you.
pub fn load(path: &Path) -> (Progress, Option<String>) {
    let Ok(text) = fs::read_to_string(path) else {
        return (Progress::default(), None);
    };
    match serde_json::from_str::<Progress>(&text) {
        Ok(p) => (p, None),
        Err(err) => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let aside = path.with_extension(format!("json.bad-{stamp}"));
            let note = match fs::rename(path, &aside) {
                Ok(()) => format!(
                    "progress file was unreadable ({err}), moved to {}",
                    aside.display()
                ),
                Err(e) => {
                    format!("progress file is unreadable ({err}) and could not be moved ({e})")
                }
            };
            (Progress::default(), Some(note))
        }
    }
}

// write to a temp file then rename, so a crash mid-save can't truncate progress
pub fn save(path: &Path, p: &Progress) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(p)?)?;
    fs::rename(&tmp, path)
}

// utc calendar date from unix ms (howard hinnant's days-to-civil)
pub fn day_key(ms: u64) -> String {
    let z = (ms / 86_400_000) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

// days in a row with practice, ending today. a streak that ended yesterday
// still counts until today is over.
pub fn streak(days: &BTreeMap<String, u64>, now_ms: u64) -> u32 {
    const DAY: u64 = 86_400_000;
    let did = |ms: u64| days.get(&day_key(ms)).is_some_and(|t| *t > 0);
    // start from yesterday when today has nothing yet
    let mut at = if did(now_ms) {
        now_ms
    } else {
        now_ms.saturating_sub(DAY)
    };
    let mut n = 0;
    while at >= DAY && did(at) {
        n += 1;
        at -= DAY;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::KeyStat;

    const DAY: u64 = 86_400_000;
    const NOW: u64 = 1_794_009_600_000 + 3_600_000; // 2026-11-07 01:00 utc

    fn practiced(days_ago: &[u64]) -> BTreeMap<String, u64> {
        days_ago
            .iter()
            .map(|d| (day_key(NOW - d * DAY), 60_000))
            .collect()
    }

    #[test]
    fn streak_counts_consecutive_days_ending_today() {
        assert_eq!(streak(&BTreeMap::new(), NOW), 0);
        assert_eq!(streak(&practiced(&[0, 1, 2]), NOW), 3);
        assert_eq!(streak(&practiced(&[0, 2, 3]), NOW), 1);
    }

    #[test]
    fn streak_survives_until_today_ends() {
        assert_eq!(streak(&practiced(&[1, 2]), NOW), 2);
        assert_eq!(streak(&practiced(&[3]), NOW), 0);
    }

    #[test]
    fn days_with_no_time_do_not_count() {
        let mut d = practiced(&[1]);
        d.insert(day_key(NOW), 0);
        assert_eq!(streak(&d, NOW), 1);
    }

    fn round(wpm: u32) -> Round {
        Round {
            mode: "base".into(),
            wpm,
            acc: 97,
            at_ms: 1_762_500_000_000,
        }
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/progress.json");
        let mut p = Progress::default();
        p.stats.insert(
            'f',
            KeyStat {
                n: 9,
                e: 1,
                ms: 1800,
            },
        );
        p.boxes.insert("zsh:ctrl+r".into(), 2);
        p.add_round(round(41), 60_000);
        save(&path, &p).unwrap();
        assert_eq!(load(&path), (p, None));
    }

    #[test]
    fn missing_file_is_a_fresh_start() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load(&dir.path().join("nope.json")),
            (Progress::default(), None)
        );
    }

    #[test]
    fn corrupt_file_is_moved_aside_not_lost() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("progress.json");
        fs::write(&path, "{ not json").unwrap();
        let (p, warn) = load(&path);
        assert_eq!(p, Progress::default());
        assert!(warn.unwrap().contains("progress.json.bad"));
        assert!(!path.exists());
        let moved = fs::read_dir(dir.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(fs::read_to_string(moved).unwrap(), "{ not json");
    }

    #[test]
    fn older_file_missing_fields_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("progress.json");
        fs::write(
            &path,
            r#"{"version":1,"stats":{"a":{"n":5,"e":0,"ms":900}}}"#,
        )
        .unwrap();
        let (p, warn) = load(&path);
        assert_eq!(warn, None);
        assert_eq!(p.stats[&'a'].n, 5);
        assert!(p.rounds.is_empty());
    }

    #[test]
    fn add_round_tracks_practice_time_per_day_and_caps_history() {
        let mut p = Progress::default();
        for _ in 0..(MAX_ROUNDS + 5) {
            p.add_round(round(30), 1000);
        }
        assert_eq!(p.rounds.len(), MAX_ROUNDS);
        assert_eq!(
            p.days[&day_key(round(0).at_ms)],
            1000 * (MAX_ROUNDS as u64 + 5)
        );
    }

    #[test]
    fn day_key_matches_known_dates() {
        assert_eq!(day_key(0), "1970-01-01");
        assert_eq!(day_key(951_782_400_000), "2000-02-29");
        assert_eq!(day_key(1_794_009_600_000), "2026-11-07");
    }
}
