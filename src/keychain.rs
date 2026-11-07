// the real Secrets: the os keychain (macos keychain, windows credential
// manager, secret service on linux), one entry named after the app
use crate::account::Secrets;
use crate::brand::NAME;

pub struct Keychain {
    entry: keyring::Entry,
}

impl Keychain {
    pub fn new() -> Result<Keychain, String> {
        let entry = keyring::Entry::new(NAME, "github").map_err(|e| e.to_string())?;
        Ok(Keychain { entry })
    }
}

impl Secrets for Keychain {
    fn get(&self) -> Result<Option<String>, String> {
        match self.entry.get_password() {
            Ok(t) => Ok(Some(t)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn set(&self, token: &str) -> Result<(), String> {
        self.entry.set_password(token).map_err(|e| e.to_string())
    }
    fn delete(&self) -> Result<(), String> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // keyring's in-memory mock store, so tests never touch the real keychain.
    // no other test may build a Keychain, the store choice is process wide.
    #[test]
    fn missing_entries_are_none_and_deleting_twice_is_fine() {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        let k = Keychain::new().unwrap();
        assert_eq!(k.get(), Ok(None));
        k.set("gho_x").unwrap();
        assert_eq!(k.get(), Ok(Some("gho_x".to_string())));
        k.delete().unwrap();
        assert_eq!(k.get(), Ok(None));
        assert_eq!(k.delete(), Ok(()));
    }
}
