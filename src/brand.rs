// the app's name and home in one place, so a rename is a one-line change
pub const NAME: &str = "keyup";
pub const REPO: &str = "github.com/dhitalkamal/keyup";
// the name before the rename: its folders and keychain entry are carried over
pub const OLD_NAME: &str = "homerow";
// github oauth app behind the profile badge (device flow, no client secret).
// empty until the app is registered; the env var overrides it for testing.
pub const GITHUB_CLIENT_ID: &str = "";
pub const CLIENT_ID_ENV: &str = "KEYUP_GITHUB_CLIENT_ID";

#[cfg(test)]
mod tests {
    use super::*;

    // pinned on purpose: everything else reads these, so only this test
    // notices if the name changes by accident
    #[test]
    fn the_app_is_called_keyup() {
        assert_eq!(NAME, "keyup");
        assert_eq!(REPO, "github.com/dhitalkamal/keyup");
        assert_eq!(CLIENT_ID_ENV, "KEYUP_GITHUB_CLIENT_ID");
        assert_eq!(OLD_NAME, "homerow");
    }

    #[test]
    fn files_live_under_the_app_name() {
        assert!(
            crate::store::default_path().ends_with("keyup/progress.json"),
            "{}",
            crate::store::default_path().display()
        );
    }
}
