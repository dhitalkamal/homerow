// where the app keeps its files: $XDG_DATA_HOME or ~/.local/share for
// progress, $XDG_CONFIG_HOME or ~/.config for your decks. one folder per name,
// so the old name's folders can be found for the rename migration.
use std::ffi::OsString;
use std::path::PathBuf;

// the pure part, so it can be tested without touching the environment
fn base(xdg: Option<OsString>, home: Option<OsString>, fallback: &str) -> PathBuf {
    match xdg {
        Some(x) if !x.is_empty() => PathBuf::from(x),
        _ => PathBuf::from(home.unwrap_or_default()).join(fallback),
    }
}

pub fn data_dir(name: &str) -> PathBuf {
    base(
        std::env::var_os("XDG_DATA_HOME"),
        std::env::var_os("HOME"),
        ".local/share",
    )
    .join(name)
}

pub fn config_dir(name: &str) -> PathBuf {
    base(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
        ".config",
    )
    .join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_wins_when_set() {
        let b = base(
            Some("/x/data".into()),
            Some("/home/me".into()),
            ".local/share",
        );
        assert_eq!(b, PathBuf::from("/x/data"));
    }

    #[test]
    fn home_is_the_fallback() {
        let b = base(None, Some("/home/me".into()), ".local/share");
        assert_eq!(b, PathBuf::from("/home/me/.local/share"));
    }

    #[test]
    fn an_empty_xdg_value_counts_as_unset() {
        // the xdg spec says an empty value means use the default
        let b = base(Some("".into()), Some("/home/me".into()), ".config");
        assert_eq!(b, PathBuf::from("/home/me/.config"));
    }
}
