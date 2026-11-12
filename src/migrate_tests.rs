use super::*;
use std::cell::RefCell;
use std::fs;

fn write(dir: &Path, name: &str, text: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(name), text).unwrap();
}

#[test]
fn copies_the_old_folder_when_the_new_one_is_missing() {
    let t = tempfile::tempdir().unwrap();
    let (old, new) = (t.path().join("homerow"), t.path().join("keyup"));
    write(&old, "progress.json", "{\"rounds\":[]}");
    write(&old, "github.json", "{}");
    assert_eq!(copy_dir_once(&old, &new).unwrap(), Some(2));
    assert_eq!(
        fs::read_to_string(new.join("progress.json")).unwrap(),
        "{\"rounds\":[]}"
    );
    assert!(new.join("github.json").exists());
    // the old folder is a backup: still complete
    assert!(old.join("progress.json").exists() && old.join("github.json").exists());
}

#[test]
fn never_touches_an_existing_new_folder() {
    let t = tempfile::tempdir().unwrap();
    let (old, new) = (t.path().join("homerow"), t.path().join("keyup"));
    write(&old, "progress.json", "old");
    write(&new, "progress.json", "new");
    assert_eq!(copy_dir_once(&old, &new).unwrap(), None);
    assert_eq!(
        fs::read_to_string(new.join("progress.json")).unwrap(),
        "new"
    );
}

#[test]
fn nothing_to_do_without_an_old_folder() {
    let t = tempfile::tempdir().unwrap();
    let new = t.path().join("keyup");
    assert_eq!(
        copy_dir_once(&t.path().join("homerow"), &new).unwrap(),
        None
    );
    assert!(
        !new.exists(),
        "a fresh install must not get an empty folder"
    );
}

#[test]
fn copies_files_only_not_subfolders() {
    let t = tempfile::tempdir().unwrap();
    let (old, new) = (t.path().join("homerow"), t.path().join("keyup"));
    write(&old, "decks.txt", "[Figma]");
    fs::create_dir_all(old.join("stuff")).unwrap();
    assert_eq!(copy_dir_once(&old, &new).unwrap(), Some(1));
    assert!(new.join("decks.txt").exists());
    assert!(!new.join("stuff").exists());
}

#[test]
fn a_crashed_earlier_copy_is_finished_not_trusted() {
    let t = tempfile::tempdir().unwrap();
    let (old, new) = (t.path().join("homerow"), t.path().join("keyup"));
    write(&old, "progress.json", "full");
    // a half copy from a run that died: hidden, so `new` still looks missing
    write(&t.path().join(".keyup.copying"), "progress.json", "ha");
    assert_eq!(copy_dir_once(&old, &new).unwrap(), Some(1));
    assert_eq!(
        fs::read_to_string(new.join("progress.json")).unwrap(),
        "full"
    );
    assert!(!t.path().join(".keyup.copying").exists());
}

// a keychain entry in memory
#[derive(Default)]
struct Entry {
    token: RefCell<Option<String>>,
    // set and delete fail, like a keychain prompt you denied
    locked: bool,
    // set "works" but the value does not read back
    lossy: bool,
}

impl Entry {
    fn with(token: &str) -> Entry {
        Entry {
            token: RefCell::new(Some(token.to_string())),
            ..Entry::default()
        }
    }
    fn get_now(&self) -> Option<String> {
        self.token.borrow().clone()
    }
}

impl Secrets for Entry {
    fn get(&self) -> Result<Option<String>, String> {
        Ok(self.token.borrow().clone())
    }
    fn set(&self, token: &str) -> Result<(), String> {
        if self.locked {
            return Err("denied".to_string());
        }
        if !self.lossy {
            *self.token.borrow_mut() = Some(token.to_string());
        }
        Ok(())
    }
    fn delete(&self) -> Result<(), String> {
        if self.locked {
            return Err("denied".to_string());
        }
        *self.token.borrow_mut() = None;
        Ok(())
    }
}

#[test]
fn the_token_moves_and_the_old_entry_goes() {
    let (old, new) = (Entry::with("ghp_abc"), Entry::default());
    assert_eq!(carry_token(&old, &new), Ok(true));
    assert_eq!(new.get_now().as_deref(), Some("ghp_abc"));
    assert_eq!(old.get_now(), None, "no stray working token left behind");
}

#[test]
fn no_old_token_means_nothing_to_move() {
    let (old, new) = (Entry::default(), Entry::default());
    assert_eq!(carry_token(&old, &new), Ok(false));
    assert_eq!(new.get_now(), None);
}

#[test]
fn a_token_already_under_the_new_name_is_kept() {
    let (old, new) = (Entry::with("ghp_old"), Entry::with("ghp_new"));
    assert_eq!(carry_token(&old, &new), Ok(false));
    assert_eq!(new.get_now().as_deref(), Some("ghp_new"));
}

#[test]
fn the_old_token_stays_when_the_new_entry_cannot_be_written() {
    let old = Entry::with("ghp_abc");
    let new = Entry {
        locked: true,
        ..Entry::default()
    };
    assert!(carry_token(&old, &new).is_err());
    assert_eq!(old.get_now().as_deref(), Some("ghp_abc"), "login not lost");
}

#[test]
fn the_old_token_stays_when_the_new_one_does_not_read_back() {
    let old = Entry::with("ghp_abc");
    let new = Entry {
        lossy: true,
        ..Entry::default()
    };
    assert!(carry_token(&old, &new).is_err());
    assert_eq!(old.get_now().as_deref(), Some("ghp_abc"));
}

#[test]
fn a_stuck_old_entry_is_reported() {
    let old = Entry {
        token: RefCell::new(Some("ghp_abc".to_string())),
        locked: true,
        ..Entry::default()
    };
    let new = Entry::default();
    let e = carry_token(&old, &new).unwrap_err();
    assert!(e.contains("homerow"), "{e}");
    // the new entry works, so the login itself is not lost
    assert_eq!(new.get_now().as_deref(), Some("ghp_abc"));
}

fn places(root: &Path) -> Places {
    Places {
        old_data: root.join("share/homerow"),
        new_data: root.join("share/keyup"),
        old_config: root.join("config/homerow"),
        new_config: root.join("config/keyup"),
    }
}

fn keys_with(old: Entry, new: Entry) -> impl FnOnce() -> Keys {
    move || {
        Ok((
            Box::new(old) as Box<dyn Secrets>,
            Box::new(new) as Box<dyn Secrets>,
        ))
    }
}

fn no_keys() -> Keys {
    panic!("the keychain must not be opened")
}

#[test]
fn a_signed_in_user_gets_data_token_and_decks() {
    let t = tempfile::tempdir().unwrap();
    let p = places(t.path());
    write(&p.old_data, "progress.json", "{}");
    write(&p.old_data, "github.json", "{}");
    write(&p.old_config, "decks.txt", "[Figma]");
    let mut said = Vec::new();
    let r = run(
        &p,
        keys_with(Entry::with("ghp_abc"), Entry::default()),
        &mut |l| said.push(l.to_string()),
    );
    assert_eq!(r, Ok(()));
    assert!(p.new_data.join("github.json").exists());
    assert!(p.new_config.join("decks.txt").exists());
    let all = said.join("\n");
    assert!(all.contains("progress") && all.contains("decks"), "{all}");
    // the warning comes before macos asks about the keychain
    let warn = said
        .iter()
        .position(|l| l.contains("macOS may ask"))
        .unwrap();
    let moved = said.iter().position(|l| l.contains("login moved")).unwrap();
    assert!(warn < moved, "{all}");
}

#[test]
fn never_signed_in_means_the_keychain_is_never_opened() {
    let t = tempfile::tempdir().unwrap();
    let p = places(t.path());
    write(&p.old_data, "progress.json", "{}");
    assert_eq!(run(&p, no_keys, &mut |_| {}), Ok(()));
    assert!(p.new_data.join("progress.json").exists());
}

#[test]
fn later_runs_do_nothing_and_say_nothing() {
    let t = tempfile::tempdir().unwrap();
    let p = places(t.path());
    write(&p.old_data, "github.json", "{}");
    write(&p.new_data, "github.json", "{}");
    write(&p.new_config, "decks.txt", "");
    let mut said = Vec::new();
    assert_eq!(run(&p, no_keys, &mut |l| said.push(l.to_string())), Ok(()));
    assert!(said.is_empty(), "{said:?}");
}

#[test]
fn a_fresh_install_does_nothing() {
    let t = tempfile::tempdir().unwrap();
    let p = places(t.path());
    let mut said = Vec::new();
    assert_eq!(run(&p, no_keys, &mut |l| said.push(l.to_string())), Ok(()));
    assert!(said.is_empty() && !p.new_data.exists() && !p.new_config.exists());
}

#[test]
fn a_failed_data_copy_stops_everything() {
    let t = tempfile::tempdir().unwrap();
    let p = places(t.path());
    write(&p.old_data, "github.json", "{}");
    // a file where the new folder's parent should be makes the copy fail
    fs::write(t.path().join("share-blocker"), "").unwrap();
    let p = Places {
        new_data: t.path().join("share-blocker/keyup"),
        ..p
    };
    let e = run(&p, no_keys, &mut |_| {}).unwrap_err();
    assert!(e.contains("could not copy"), "{e}");
}

#[test]
fn a_keychain_problem_is_said_but_does_not_stop_the_app() {
    let t = tempfile::tempdir().unwrap();
    let p = places(t.path());
    write(&p.old_data, "github.json", "{}");
    let mut said = Vec::new();
    let r = run(&p, || Err("keychain locked".to_string()), &mut |l| {
        said.push(l.to_string())
    });
    assert_eq!(r, Ok(()));
    let all = said.join("\n");
    assert!(
        all.contains("keychain locked") && all.contains("login --token"),
        "{all}"
    );
}
