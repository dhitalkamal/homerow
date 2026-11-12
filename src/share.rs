// the share card copied after a round: what you did and where to get the app
use crate::brand::{NAME, REPO};
use crate::history::{mode_label, test_of};
use crate::store::Round;

pub fn card(r: &Round, streak: u32) -> String {
    let what = format!("{} {}", mode_label(&r.mode), test_of(r));
    let first = format!(
        "{NAME} {}: {} wpm, {}% accuracy",
        what.trim_end(),
        r.wpm,
        r.acc
    );
    let second = match streak {
        0 => format!("try it: {REPO}"),
        1 => format!("streak 1 day, try it: {REPO}"),
        n => format!("streak {n} days, try it: {REPO}"),
    };
    format!("{first}\n{second}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round(mode: &str, test: Option<&str>) -> Round {
        Round {
            mode: mode.into(),
            wpm: 52,
            acc: 97,
            at_ms: 0,
            test: test.map(Into::into),
        }
    }

    #[test]
    fn daily_card_names_the_day_and_invites_friends() {
        let c = card(&round("daily", Some("2026-11-07")), 4);
        assert_eq!(
            c,
            format!("{NAME} daily 2026-11-07: 52 wpm, 97% accuracy\nstreak 4 days, try it: {REPO}")
        );
    }

    #[test]
    fn baseline_card_says_which_test() {
        let c = card(&round("base", Some("30s +punct")), 1);
        assert!(
            c.starts_with(&format!("{NAME} baseline 30s +punct: 52 wpm")),
            "{c}"
        );
        assert!(c.contains("streak 1 day,"), "{c}");
    }

    #[test]
    fn no_streak_line_part_when_there_is_none() {
        let c = card(&round("keys", None), 0);
        assert!(c.starts_with(&format!("{NAME} weak keys: 52 wpm")), "{c}");
        assert!(
            c.ends_with(&format!("try it: {REPO}")) && !c.contains("streak"),
            "{c}"
        );
    }
}
