// tests for the progress file: saving, loading, settings, streaks, dates
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
    assert_eq!(p.density, Density::Comfortable);
    assert_eq!(p.settings, Settings::default());
}

#[test]
fn keyboard_starts_hidden_with_hands_and_colors_ready() {
    let s = Settings::default();
    assert!(!s.keyboard && s.hands && s.finger_colors && !s.heatmap && !s.hide_labels);
}

#[test]
fn settings_are_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    let mut p = Progress::default();
    p.settings.keyboard = true;
    p.settings.heatmap = true;
    save(&path, &p).unwrap();
    assert_eq!(load(&path).0.settings, p.settings);
}

#[test]
fn density_steps_and_stops_at_the_ends() {
    use Density::*;
    assert_eq!(Comfortable.step(true), Spacious);
    assert_eq!(Spacious.step(true), Spacious);
    assert_eq!(Comfortable.step(false), Compact);
    assert_eq!(Compact.step(false), Compact);
}

#[test]
fn density_is_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    let p = Progress {
        density: Density::Spacious,
        ..Progress::default()
    };
    save(&path, &p).unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains("\"spacious\""));
    assert_eq!(load(&path).0.density, Density::Spacious);
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
