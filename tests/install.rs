// runs the real install.sh against a fake release folder served over file://
// so the install, checksum and fallback paths are checked without a network
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ASSET: &str = "keyup-aarch64-apple-darwin.tar.gz";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("install.sh")
}

// a release folder with a tiny stand-in binary, its tarball and checksum
fn fake_release(dir: &Path, tamper: bool) {
    let stage = dir.join("stage");
    fs::create_dir_all(&stage).unwrap();
    fs::write(stage.join("keyup"), "#!/bin/sh\necho keyup test build\n").unwrap();
    run(
        "chmod",
        &["755", stage.join("keyup").to_str().unwrap()],
        &stage,
    );
    fs::write(stage.join("LICENSE"), "MIT\n").unwrap();
    run(
        "tar",
        &["czf", dir.join(ASSET).to_str().unwrap(), "keyup", "LICENSE"],
        &stage,
    );
    let sum = run("shasum", &["-a", "256", ASSET], dir);
    let mut line = String::from_utf8(sum.stdout).unwrap();
    if tamper {
        line = format!("{}{}", "0".repeat(64), &line[64..]);
    }
    fs::write(dir.join(format!("{ASSET}.sha256")), line).unwrap();
}

fn run(cmd: &str, args: &[&str], cwd: &Path) -> Output {
    let out = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{cmd} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

// runs install.sh with a clean PATH so a real cargo or keyup is never used
fn install(release: &Path, bin: &Path, platform: &str) -> Output {
    install_with(release, bin, platform, &[])
}

// same, with extra environment (SHELL, PATH, opt-outs); HOME is the temp dir
fn install_with(release: &Path, bin: &Path, platform: &str, extra: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new("sh");
    cmd.arg(script())
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("HOME", release)
        .env("KEYUP_BASE_URL", format!("file://{}", release.display()))
        .env("KEYUP_INSTALL_DIR", bin)
        .env("KEYUP_PLATFORM", platform);
    for (k, v) in extra {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn installs_the_release_binary_on_apple_silicon() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    let out = install(t.path(), &bin, "Darwin-arm64");
    assert!(out.status.success(), "{}", text(&out));
    let ran = Command::new(bin.join("keyup")).output().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout).trim(),
        "keyup test build"
    );
    assert!(text(&out).contains("installed keyup"), "{}", text(&out));
}

#[test]
fn checksum_mismatch_installs_nothing_and_keeps_the_old_version() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), true);
    let bin = t.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("keyup"), "old version").unwrap();
    let out = install(t.path(), &bin, "Darwin-arm64");
    assert!(!out.status.success());
    assert!(text(&out).contains("checksum"), "{}", text(&out));
    assert_eq!(
        fs::read_to_string(bin.join("keyup")).unwrap(),
        "old version"
    );
}

#[test]
fn unsupported_platform_without_cargo_explains_how_to_get_rust() {
    let t = tempfile::tempdir().unwrap();
    let out = install(t.path(), &t.path().join("bin"), "Linux-x86_64");
    assert!(!out.status.success());
    assert!(text(&out).contains("rustup"), "{}", text(&out));
    assert!(!t.path().join("bin").join("keyup").exists());
}

#[test]
fn says_how_to_add_the_folder_to_path() {
    // an unknown shell gets printed instructions (sh fills SHELL in from the
    // login shell when it is missing, so the shell is named here)
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    let out = install_with(t.path(), &bin, "Darwin-arm64", &[("SHELL", "/bin/tcsh")]);
    assert!(text(&out).contains("export PATH="), "{}", text(&out));
}

#[test]
fn zsh_profile_gets_the_path_line_once() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    for _ in 0..2 {
        let out = install_with(t.path(), &bin, "Darwin-arm64", &[("SHELL", "/bin/zsh")]);
        assert!(out.status.success(), "{}", text(&out));
    }
    let rc = fs::read_to_string(t.path().join(".zshrc")).unwrap();
    assert_eq!(rc.matches(bin.to_str().unwrap()).count(), 1, "{rc}");
    assert!(
        rc.contains("export PATH=") && rc.contains("keyup installer"),
        "{rc}"
    );
}

#[test]
fn bash_on_macos_uses_bash_profile() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    let out = install_with(t.path(), &bin, "Darwin-arm64", &[("SHELL", "/bin/bash")]);
    assert!(text(&out).contains(".bash_profile"), "{}", text(&out));
    assert!(
        fs::read_to_string(t.path().join(".bash_profile"))
            .unwrap()
            .contains(bin.to_str().unwrap())
    );
}

#[test]
fn fish_uses_fish_add_path() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    install_with(
        t.path(),
        &bin,
        "Darwin-arm64",
        &[("SHELL", "/opt/homebrew/bin/fish")],
    );
    let cfg = fs::read_to_string(t.path().join(".config/fish/config.fish")).unwrap();
    assert!(
        cfg.contains("fish_add_path") && cfg.contains(bin.to_str().unwrap()),
        "{cfg}"
    );
}

#[test]
fn opt_out_and_unknown_shells_leave_profiles_alone() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    let out = install_with(
        t.path(),
        &bin,
        "Darwin-arm64",
        &[("SHELL", "/bin/zsh"), ("KEYUP_NO_MODIFY_PATH", "1")],
    );
    assert!(!t.path().join(".zshrc").exists());
    assert!(text(&out).contains("export PATH="), "{}", text(&out));
    install_with(t.path(), &bin, "Darwin-arm64", &[("SHELL", "/bin/tcsh")]);
    assert!(!t.path().join(".tcshrc").exists() && !t.path().join(".zshrc").exists());
}

#[test]
fn folder_already_on_path_changes_nothing() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    let path = format!("{}:/usr/bin:/bin", bin.display());
    install_with(
        t.path(),
        &bin,
        "Darwin-arm64",
        &[("SHELL", "/bin/zsh"), ("PATH", &path)],
    );
    assert!(!t.path().join(".zshrc").exists());
}

#[test]
fn leaves_an_old_homerow_binary_and_says_so() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("homerow"), "old homerow").unwrap();
    // one run ends at the profile step, one exits early on PATH
    let on_path = format!("{}:/usr/bin:/bin", bin.display());
    let runs: [&[(&str, &str)]; 2] = [
        &[("SHELL", "/bin/tcsh")],
        &[("SHELL", "/bin/zsh"), ("PATH", &on_path)],
    ];
    for extra in runs {
        let out = install_with(t.path(), &bin, "Darwin-arm64", extra);
        assert!(out.status.success(), "{}", text(&out));
        let note = format!(
            "the old homerow command is still at {}/homerow",
            bin.display()
        );
        assert!(text(&out).contains(&note), "{}", text(&out));
        assert_eq!(
            fs::read_to_string(bin.join("homerow")).unwrap(),
            "old homerow"
        );
    }
}

#[test]
fn no_old_binary_means_no_note() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let out = install(t.path(), &t.path().join("bin"), "Darwin-arm64");
    assert!(!text(&out).contains("old homerow"), "{}", text(&out));
}

#[test]
fn profile_from_the_old_installer_gets_no_second_path_line() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    // exactly what the homerow installer wrote
    let old = format!(
        "\n# added by the homerow installer\nexport PATH=\"{}:$PATH\"\n",
        bin.display()
    );
    fs::write(t.path().join(".zshrc"), &old).unwrap();
    let out = install_with(t.path(), &bin, "Darwin-arm64", &[("SHELL", "/bin/zsh")]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(fs::read_to_string(t.path().join(".zshrc")).unwrap(), old);
}
