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
        test: None,
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
    // old combined key stats now live under baseline
    assert_eq!(p.mode_stats["base"][&'a'].n, 5);
    assert!(p.rounds.is_empty());
    // a version 1 file is migrated to auto size
    assert_eq!(p.density, Density::Auto);
    assert_eq!(p.settings, Settings::default());
}

#[test]
fn keyboard_starts_hidden_with_hands_and_colors_ready() {
    let s = Settings::default();
    assert!(!s.keyboard && s.hands && s.finger_colors && !s.heatmap && !s.hide_labels);
    assert!(
        !s.big_text,
        "big text is opt-in, 4x is too big as a default"
    );
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
    assert_eq!(Spacious.step(true), Large);
    assert_eq!(Large.step(true), Large);
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

#[test]
fn auto_is_the_default_density() {
    assert_eq!(Progress::default().density, Density::Auto);
}

#[test]
fn a_chosen_size_ignores_the_terminal() {
    use Density::*;
    for d in [Compact, Comfortable, Spacious] {
        assert_eq!(d.resolve(60, 20), d);
        assert_eq!(d.resolve(200, 80), d);
    }
}

#[test]
fn version_1_files_switch_to_auto_size() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    fs::write(&path, r#"{"version":1,"density":"comfortable"}"#).unwrap();
    let (p, _) = load(&path);
    assert_eq!((p.density, p.version), (Density::Auto, VERSION));
    fs::write(&path, r#"{"version":2,"density":"spacious"}"#).unwrap();
    assert_eq!(load(&path).0.density, Density::Spacious);
}

#[test]
fn large_is_the_top_size_step() {
    use Density::*;
    assert_eq!(Spacious.step(true), Large);
    assert_eq!(Large.step(true), Large);
    assert_eq!(Large.step(false), Spacious);
}

#[test]
fn auto_decides_mostly_by_width() {
    use Density::*;
    assert_eq!(Auto.resolve(100, 40), Comfortable);
    assert_eq!(Auto.resolve(79, 40), Compact);
    assert_eq!(Auto.resolve(100, 19), Compact);
    assert_eq!(Auto.resolve(110, 30), Spacious);
    assert_eq!(Auto.resolve(139, 60), Spacious);
    assert_eq!(Auto.resolve(140, 30), Large);
    // the window from the screenshot
    assert_eq!(Auto.resolve(151, 43), Large);
}

#[test]
fn zoom_starts_at_normal_and_is_saved() {
    assert_eq!(Settings::default().zoom, 1);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    let mut p = Progress::default();
    p.settings.zoom = 3;
    save(&path, &p).unwrap();
    assert_eq!(load(&path).0.settings.zoom, 3);
}

#[test]
fn version_2_key_stats_move_into_the_baseline_bucket() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    fs::write(
        &path,
        r#"{"version":2,"stats":{"a":{"n":9,"e":1,"ms":900}}}"#,
    )
    .unwrap();
    let (p, _) = load(&path);
    assert_eq!(p.version, VERSION);
    assert_eq!(p.mode_stats["base"][&'a'].n, 9);
    assert!(
        p.stats.is_empty(),
        "old combined stats are moved, not kept twice"
    );
}

#[test]
fn per_mode_key_stats_are_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    let mut p = Progress::default();
    p.mode_stats.entry("code".into()).or_default().insert(
        '{',
        KeyStat {
            n: 6,
            e: 2,
            ms: 1200,
        },
    );
    save(&path, &p).unwrap();
    assert_eq!(load(&path).0.mode_stats["code"][&'{'].e, 2);
}

#[test]
fn mistakes_count_up_down_and_are_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    let mut p = Progress::default();
    p.note_mistake("their");
    p.note_mistake("their");
    p.note_mistake("cat");
    p.note_clean("cat");
    assert_eq!(p.mistakes.get("their"), Some(&2));
    assert!(!p.mistakes.contains_key("cat"), "fixed words drop off");
    save(&path, &p).unwrap();
    assert_eq!(load(&path).0.mistakes["their"], 2);
}

#[test]
fn mistakes_list_is_capped() {
    let mut p = Progress::default();
    for i in 0..(MAX_MISTAKES + 20) {
        p.note_mistake(&format!("w{i}"));
    }
    assert_eq!(p.mistakes.len(), MAX_MISTAKES);
}

#[test]
fn goal_streak_counts_only_days_that_met_the_goal() {
    let mut d = BTreeMap::new();
    d.insert(day_key(NOW), 20 * 60_000);
    d.insert(day_key(NOW - DAY), 5 * 60_000);
    d.insert(day_key(NOW - 2 * DAY), 20 * 60_000);
    assert_eq!(
        goal_streak(&d, NOW, 15),
        1,
        "yesterday was short of 15 minutes"
    );
    assert_eq!(goal_streak(&d, NOW, 0), 3, "goal off: any practice counts");
}

#[test]
fn daily_goal_defaults_to_15_minutes_and_is_saved() {
    assert_eq!(Settings::default().goal_min, 15);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    let mut p = Progress::default();
    p.settings.goal_min = 30;
    save(&path, &p).unwrap();
    assert_eq!(load(&path).0.settings.goal_min, 30);
}

#[test]
fn test_options_default_to_a_plain_minute_and_old_rounds_have_no_label() {
    let s = Settings::default();
    assert_eq!(s.test, crate::baseline::Length::Time(60));
    assert!(!s.punctuation && !s.numbers);
    let r: Round = serde_json::from_str(r#"{"mode":"base","wpm":40,"acc":95,"at_ms":1}"#).unwrap();
    assert_eq!(r.test, None);
}

#[test]
fn letter_pairs_are_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    let mut p = Progress::default();
    assert!(p.pairs.is_empty());
    p.pairs.insert(
        "th".into(),
        KeyStat {
            n: 7,
            e: 1,
            ms: 1400,
        },
    );
    save(&path, &p).unwrap();
    assert_eq!(load(&path).0.pairs["th"].n, 7);
}
