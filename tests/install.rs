// runs the real install.sh against a fake release folder served over file://
// so the install, checksum and fallback paths are checked without a network
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ASSET: &str = "homerow-aarch64-apple-darwin.tar.gz";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("install.sh")
}

// a release folder with a tiny stand-in binary, its tarball and checksum
fn fake_release(dir: &Path, tamper: bool) {
    let stage = dir.join("stage");
    fs::create_dir_all(&stage).unwrap();
    fs::write(stage.join("homerow"), "#!/bin/sh\necho homerow test build\n").unwrap();
    run("chmod", &["755", stage.join("homerow").to_str().unwrap()], &stage);
    fs::write(stage.join("LICENSE"), "MIT\n").unwrap();
    run("tar", &["czf", dir.join(ASSET).to_str().unwrap(), "homerow", "LICENSE"], &stage);
    let sum = run("shasum", &["-a", "256", ASSET], dir);
    let mut line = String::from_utf8(sum.stdout).unwrap();
    if tamper {
        line = format!("{}{}", "0".repeat(64), &line[64..]);
    }
    fs::write(dir.join(format!("{ASSET}.sha256")), line).unwrap();
}

fn run(cmd: &str, args: &[&str], cwd: &Path) -> Output {
    let out = Command::new(cmd).args(args).current_dir(cwd).output().unwrap();
    assert!(out.status.success(), "{cmd} failed: {}", String::from_utf8_lossy(&out.stderr));
    out
}

// runs install.sh with a clean PATH so a real cargo or homerow is never used
fn install(release: &Path, bin: &Path, platform: &str) -> Output {
    Command::new("sh")
        .arg(script())
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("HOME", release)
        .env("HOMEROW_BASE_URL", format!("file://{}", release.display()))
        .env("HOMEROW_INSTALL_DIR", bin)
        .env("HOMEROW_PLATFORM", platform)
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

#[test]
fn installs_the_release_binary_on_apple_silicon() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    let out = install(t.path(), &bin, "Darwin-arm64");
    assert!(out.status.success(), "{}", text(&out));
    let ran = Command::new(bin.join("homerow")).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), "homerow test build");
    assert!(text(&out).contains("installed homerow"), "{}", text(&out));
}

#[test]
fn checksum_mismatch_installs_nothing_and_keeps_the_old_version() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), true);
    let bin = t.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("homerow"), "old version").unwrap();
    let out = install(t.path(), &bin, "Darwin-arm64");
    assert!(!out.status.success());
    assert!(text(&out).contains("checksum"), "{}", text(&out));
    assert_eq!(fs::read_to_string(bin.join("homerow")).unwrap(), "old version");
}

#[test]
fn unsupported_platform_without_cargo_explains_how_to_get_rust() {
    let t = tempfile::tempdir().unwrap();
    let out = install(t.path(), &t.path().join("bin"), "Linux-x86_64");
    assert!(!out.status.success());
    assert!(text(&out).contains("rustup"), "{}", text(&out));
    assert!(!t.path().join("bin").join("homerow").exists());
}

#[test]
fn says_how_to_add_the_folder_to_path() {
    let t = tempfile::tempdir().unwrap();
    fake_release(t.path(), false);
    let bin = t.path().join("bin");
    let out = install(t.path(), &bin, "Darwin-arm64");
    assert!(text(&out).contains("export PATH="), "{}", text(&out));
}
