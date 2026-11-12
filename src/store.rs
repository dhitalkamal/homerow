// progress file: ~/.local/share/keyup/progress.json (or $XDG_DATA_HOME/keyup)
use crate::stats::KeyStats;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

// bump when the shape changes in a way old files can't be read as
// 2: density gained auto, version 1 files are moved to it
// 3: key stats are kept per mode, version 2 combined stats go to baseline
pub const VERSION: u32 = 3;
const MAX_ROUNDS: usize = 200;
// the mistakes list keeps this many words, dropping the least missed
pub const MAX_MISTAKES: usize = 200;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Round {
    pub mode: String,
    pub wpm: u32,
    pub acc: u32,
    pub at_ms: u64,
    // baseline test the round ran (like "30s +punct"); None for other modes
    // and for rounds saved before test options existed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub version: u32,
    // only read from version 2 files, moved into mode_stats on load
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub stats: KeyStats,
    // key stats per round tag (base, keys, code)
    pub mode_stats: BTreeMap<String, KeyStats>,
    // words you missed and how often; the mistakes drill practices these
    pub mistakes: BTreeMap<String, u32>,
    // letter pair speed in text modes (baseline and weak keys)
    pub pairs: crate::stats::PairStats,
    // guided lessons: how many keys are unlocked (see lessons::ORDER)
    pub lesson_keys: usize,
    pub rounds: Vec<Round>,
    pub boxes: BTreeMap<String, u32>,
    // practice ms per utc day, keyed YYYY-MM-DD
    pub days: BTreeMap<String, u64>,
    // screen density, changed with ctrl+= and ctrl+-
    pub density: Density,
    pub settings: Settings,
}

// what the typing screen shows, changed on the settings tab
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub keyboard: bool,
    pub hands: bool,
    pub finger_colors: bool,
    pub heatmap: bool,
    pub hide_labels: bool,
    // typing text drawn with block letters, 4 columns by 4 rows each
    pub big_text: bool,
    // kitty text sizing scale for the typing text, 1 is normal
    pub zoom: u8,
    // minutes of practice a day that count toward the streak, 0 is off
    pub goal_min: u32,
    // baseline test: length, sentences with punctuation, numbers mixed in
    pub test: crate::baseline::Length,
    pub punctuation: bool,
    pub numbers: bool,
    // race your best: a ghost at your personal-best pace
    pub ghost: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            keyboard: false,
            hands: true,
            finger_colors: true,
            heatmap: false,
            hide_labels: false,
            big_text: false,
            zoom: 1,
            goal_min: 15,
            test: crate::baseline::Length::Time(60),
            punctuation: false,
            numbers: false,
            ghost: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    // follow the terminal size
    #[default]
    Auto,
    Compact,
    Comfortable,
    Spacious,
    // very big terminals: wide text, double size keyboard
    Large,
}

impl Density {
    // one step bigger (true) or smaller, stopping at the ends
    pub fn step(self, bigger: bool) -> Density {
        const ORDER: [Density; 4] = [
            Density::Compact,
            Density::Comfortable,
            Density::Spacious,
            Density::Large,
        ];
        let i = ORDER.iter().position(|d| *d == self).unwrap_or(1);
        ORDER[if bigger {
            (i + 1).min(ORDER.len() - 1)
        } else {
            i.saturating_sub(1)
        }]
    }
    // the size to draw at: auto picks mostly by terminal width, since the
    // layout already drops hands and keyboard when height runs short.
    // a chosen size is used as is.
    pub fn resolve(self, width: u16, height: u16) -> Density {
        match self {
            Density::Auto if width < 80 || height < 20 => Density::Compact,
            Density::Auto if width >= 140 => Density::Large,
            Density::Auto if width >= 110 => Density::Spacious,
            Density::Auto => Density::Comfortable,
            d => d,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Density::Auto => "auto",
            Density::Compact => "compact",
            Density::Comfortable => "comfortable",
            Density::Spacious => "spacious",
            Density::Large => "large",
        }
    }
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            version: VERSION,
            stats: KeyStats::new(),
            mode_stats: BTreeMap::new(),
            mistakes: BTreeMap::new(),
            pairs: crate::stats::PairStats::new(),
            lesson_keys: crate::lessons::START,
            rounds: Vec::new(),
            boxes: BTreeMap::new(),
            days: BTreeMap::new(),
            density: Density::default(),
            settings: Settings::default(),
        }
    }
}

impl Progress {
    // one more miss for this word; single letters are not worth drilling
    pub fn note_mistake(&mut self, word: &str) {
        if word.chars().count() < 2 {
            return;
        }
        if !self.mistakes.contains_key(word) && self.mistakes.len() >= MAX_MISTAKES {
            let least = self
                .mistakes
                .iter()
                .min_by_key(|(_, n)| **n)
                .map(|(w, _)| w.clone());
            if let Some(w) = least {
                self.mistakes.remove(&w);
            }
        }
        *self.mistakes.entry(word.to_string()).or_insert(0) += 1;
    }

    // typed cleanly: one step closer to dropping off the list
    pub fn note_clean(&mut self, word: &str) {
        if let Some(n) = self.mistakes.get_mut(word) {
            *n -= 1;
            if *n == 0 {
                self.mistakes.remove(word);
            }
        }
    }

    pub fn add_round(&mut self, round: Round, practice_ms: u64) {
        *self.days.entry(day_key(round.at_ms)).or_insert(0) += practice_ms;
        self.rounds.push(round);
        if self.rounds.len() > MAX_ROUNDS {
            self.rounds.drain(..self.rounds.len() - MAX_ROUNDS);
        }
    }
}

pub fn default_path() -> PathBuf {
    crate::dirs::data_dir(crate::brand::NAME).join("progress.json")
}

// a missing file is a fresh start. a corrupt one is moved aside, never deleted,
// and the second value says where it went so the app can tell you.
pub fn load(path: &Path) -> (Progress, Option<String>) {
    let Ok(text) = fs::read_to_string(path) else {
        return (Progress::default(), None);
    };
    match serde_json::from_str::<Progress>(&text) {
        Ok(mut p) => {
            // density could only be saved as the default before auto existed
            if p.version < 2 {
                p.density = Density::Auto;
            }
            // before version 3 all modes shared one set of key stats; they
            // cannot be split after the fact, so they count as baseline
            if p.version < 3 && !p.stats.is_empty() {
                let old = std::mem::take(&mut p.stats);
                let base = p.mode_stats.entry("base".into()).or_default();
                for (c, s) in old {
                    let e = base.entry(c).or_default();
                    e.n += s.n;
                    e.e += s.e;
                    e.ms += s.ms;
                }
            }
            p.version = VERSION;
            (p, None)
        }
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
#[cfg(test)]
pub fn streak(days: &BTreeMap<String, u64>, now_ms: u64) -> u32 {
    goal_streak(days, now_ms, 0)
}

// like streak, but a day only counts once it reaches the daily goal (in
// minutes); a goal of 0 counts any practice
pub fn goal_streak(days: &BTreeMap<String, u64>, now_ms: u64, goal_min: u32) -> u32 {
    let need = (goal_min as u64 * 60_000).max(1);
    const DAY: u64 = 86_400_000;
    let did = |ms: u64| days.get(&day_key(ms)).is_some_and(|t| *t >= need);
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
#[path = "store_tests.rs"]
mod tests;
