// the app's name and home in one place, so a rename is a one-line change
pub const NAME: &str = "homerow";
pub const REPO: &str = "github.com/dhitalkamal/homerow";
// github oauth app behind the profile badge (device flow, no client secret).
// empty until the app is registered; the env var overrides it for testing.
pub const GITHUB_CLIENT_ID: &str = "";
pub const CLIENT_ID_ENV: &str = "HOMEROW_GITHUB_CLIENT_ID";
