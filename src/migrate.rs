// first run after the rename from homerow: copy the old folders into the new
// ones once, and carry the github token over to the new keychain entry. the old
// folders are never changed, so the old binary keeps working and nothing is lost.
use crate::account::Secrets;
use crate::brand::{NAME, OLD_NAME};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// the old and new folders: real ones from dirs.rs, temp ones in tests
pub struct Places {
    pub old_data: PathBuf,
    pub new_data: PathBuf,
    pub old_config: PathBuf,
    pub new_config: PathBuf,
}

// the old and new keychain entries, opened only when there is a token to carry
pub type Keys = Result<(Box<dyn Secrets>, Box<dyn Secrets>), String>;

// data first, then the token, then the decks, each said in one line. an error
// means a copy did not finish: the caller must stop before the app saves fresh
// progress into the new folder, or the copy would never be tried again.
pub fn run(
    places: &Places,
    keys: impl FnOnce() -> Keys,
    say: &mut dyn FnMut(&str),
) -> Result<(), String> {
    let copied = copy_and_say(&places.old_data, &places.new_data, "progress", say)?;
    // only someone who was signed in has a token to carry. checking the badge
    // link first keeps everyone else away from the keychain and its prompt
    let link = crate::account::link_path(&places.new_data.join("progress.json"));
    if copied && link.exists() {
        say(&format!(
            "moving your github login to {NAME}. macOS may ask once to let {NAME} \
             use the {OLD_NAME} keychain entry, allow it"
        ));
        match keys().and_then(|(old, new)| carry_token(old.as_ref(), new.as_ref())) {
            Ok(true) => say("your github login moved"),
            Ok(false) => {}
            Err(e) => say(&format!(
                "{e}. if your badge stops updating, run: {NAME} login --token"
            )),
        }
    }
    copy_and_say(&places.old_config, &places.new_config, "decks", say)?;
    Ok(())
}

fn copy_and_say(
    old: &Path,
    new: &Path,
    what: &str,
    say: &mut dyn FnMut(&str),
) -> Result<bool, String> {
    match copy_dir_once(old, new) {
        Ok(Some(_)) => {
            say(&format!(
                "copied your {OLD_NAME} {what} to {} (the old {} stays as a backup, \
                 delete it when you like)",
                new.display(),
                old.display()
            ));
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(e) => Err(format!(
            "could not copy {} to {}: {e}. nothing in {} was changed",
            old.display(),
            new.display(),
            old.display()
        )),
    }
}

// copy the regular files of `old` into `new`, only when `new` does not exist
// yet. the copy goes into a hidden sibling folder that is renamed into place at
// the end, so `new` never exists half-filled. returns how many files came over.
pub fn copy_dir_once(old: &Path, new: &Path) -> io::Result<Option<usize>> {
    if new.exists() || !old.is_dir() {
        return Ok(None);
    }
    let name = new.file_name().unwrap_or_default().to_string_lossy();
    let tmp = new.with_file_name(format!(".{name}.copying"));
    // a copy that died part way left this behind; copying over it again
    // finishes the job without deleting anything
    fs::create_dir_all(&tmp)?;
    let mut count = 0;
    for entry in fs::read_dir(old)? {
        let entry = entry?;
        // file_type does not follow symlinks, so only plain files come over
        if entry.file_type()?.is_file() {
            fs::copy(entry.path(), tmp.join(entry.file_name()))?;
            count += 1;
        }
    }
    fs::rename(&tmp, new)?;
    Ok(Some(count))
}

// move the token to the new keychain entry. the old entry is deleted only after
// the new one reads back the same token, so a failure never loses the login
// and success leaves no stray working token behind. true when one was moved.
pub fn carry_token(old: &dyn Secrets, new: &dyn Secrets) -> Result<bool, String> {
    if new.get()?.is_some() {
        return Ok(false);
    }
    let Some(token) = old.get()? else {
        return Ok(false);
    };
    new.set(&token)
        .map_err(|e| format!("could not save your github login for {NAME}: {e}"))?;
    if new.get()?.as_deref() != Some(token.as_str()) {
        return Err(format!(
            "the {NAME} keychain entry did not keep your github login"
        ));
    }
    old.delete().map_err(|e| {
        format!(
            "your github login moved, but the old {OLD_NAME} keychain entry \
             could not be removed ({e}). delete it in Keychain Access"
        )
    })?;
    Ok(true)
}

#[cfg(test)]
#[path = "migrate_tests.rs"]
mod tests;
