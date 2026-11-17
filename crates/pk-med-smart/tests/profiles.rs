//! Sessions kept per profile in the vault, and the move from one keychain
//! item per session. Synthetic sessions only; nothing reads the keychain.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use pk_cli_auth::refresh_lock::RefreshLock;
use pk_cli_auth::vault::{KeyStore, MemoryKeys, Vault};
use pk_cli_config::profile::ProfileName;
use pk_cli_core::CliError;
use pk_cli_secrets::Secret;
use pk_med_smart::{
    account, decode_session, migrate_sessions, Migration, ProfileSessions, SessionStore,
    SmartSession,
};

const PATIENT: &str = "111111111";

struct Dir(PathBuf);

impl Dir {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "pk-med-profiles-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Dir(dir)
    }

    fn vault(&self, keys: &Arc<MemoryKeys>) -> Vault {
        Vault::new(Arc::clone(keys), self.0.join("vault"))
    }

    fn lock(&self) -> RefreshLock {
        RefreshLock::at(self.0.join("refresh.lock")).wait_at_most(Duration::from_secs(5))
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn profile(name: &str) -> ProfileName {
    ProfileName::parse(name).unwrap()
}

fn stored(organization: &str, patient: &str, token: &str) -> String {
    format!(
        r#"{{"format":1,"organization":"{organization}","patient":"{patient}","fhir_base":"https://fhir.example.test","token_endpoint":"https://fhir.example.test/token","client_id":"c","scope":"openid","tokens":{{"access_token":"{token}","token_type":"Bearer","obtained_at":0}}}}"#
    )
}

fn session(organization: &str, patient: &str, token: &str) -> SmartSession {
    decode_session(&stored(organization, patient, token)).unwrap()
}

#[test]
fn each_profile_keeps_its_own_sessions() {
    let dir = Dir::new();
    let keys = Arc::new(MemoryKeys::new());
    let vault = dir.vault(&keys);
    let one = ProfileSessions::new(&vault, profile("adult1"));
    let two = ProfileSessions::new(&vault, profile("adult2"));
    one.save(&session("org-1", PATIENT, "token-one")).unwrap();
    two.save(&session("org-1", PATIENT, "token-two")).unwrap();
    two.save(&session("org-2", "222222222", "token-three"))
        .unwrap();

    let acct = account("org-1", PATIENT);
    assert_eq!(
        one.load(&acct).unwrap().unwrap().tokens.access_token,
        "token-one"
    );
    assert_eq!(
        two.load(&acct).unwrap().unwrap().tokens.access_token,
        "token-two"
    );
    assert_eq!(one.accounts().unwrap(), std::slice::from_ref(&acct));
    assert_eq!(two.accounts().unwrap().len(), 2);
    assert_eq!(
        vault.names().unwrap(),
        [
            format!("adult1/org-1/{PATIENT}"),
            format!("adult2/org-1/{PATIENT}"),
            "adult2/org-2/222222222".to_string(),
        ]
    );

    vault
        .put("_client-secret", &Secret::new("synthetic-client-secret"))
        .unwrap();
    assert_eq!(
        two.accounts().unwrap().len(),
        2,
        "a CLI's own secret is no session"
    );
    assert_eq!(two.delete_all().unwrap(), 2);
    assert!(vault.get("_client-secret").unwrap().is_some());
    assert!(two.load(&acct).unwrap().is_none());
    assert!(
        one.load(&acct).unwrap().is_some(),
        "another profile is untouched"
    );
}

#[test]
fn many_sessions_cost_one_keychain_read() {
    let dir = Dir::new();
    let keys = Arc::new(MemoryKeys::new());
    {
        let vault = dir.vault(&keys);
        for (name, patient) in [("adult1", "1"), ("adult1", "2"), ("adult2", "3")] {
            ProfileSessions::new(&vault, profile(name))
                .save(&session("org-1", patient, "t"))
                .unwrap();
        }
    }
    let reads = keys.reads();
    let vault = dir.vault(&keys);
    for (name, patient) in [("adult1", "1"), ("adult1", "2"), ("adult2", "3")] {
        let sessions = ProfileSessions::new(&vault, profile(name));
        assert!(sessions.load(&account("org-1", patient)).unwrap().is_some());
    }
    assert_eq!(keys.reads(), reads + 1, "one read for the process");
    assert_eq!(keys.writes(), 1, "one item ever written");
}

#[test]
fn an_account_of_another_shape_is_refused() {
    let dir = Dir::new();
    let keys = Arc::new(MemoryKeys::new());
    let vault = dir.vault(&keys);
    let sessions = ProfileSessions::new(&vault, profile("adult1"));
    let err = sessions.load("org-1/111").unwrap_err();
    assert!(err.to_string().contains("pk_med_smart::account"), "{err}");
}

#[test]
fn old_items_move_into_a_profile_once() {
    let dir = Dir::new();
    let keys = Arc::new(MemoryKeys::new());
    let vault = dir.vault(&keys);
    let sessions = ProfileSessions::new(&vault, profile("adult1"));
    let old = MemoryKeys::new();
    let moving = account("org-1", PATIENT);
    let kept = account("org-2", "222222222");
    let absent = account("org-3", "333333333");
    old.set(&moving, &Secret::new(stored("org-1", PATIENT, "old-token")))
        .unwrap();
    old.set(&kept, &Secret::new(stored("org-2", "222222222", "stale")))
        .unwrap();
    sessions
        .save(&session("org-2", "222222222", "newer"))
        .unwrap();

    let accounts = [moving.clone(), kept.clone(), absent];
    let done = migrate_sessions(&old, &sessions, &accounts, &dir.lock()).unwrap();
    assert_eq!(
        done,
        Migration {
            moved: 1,
            kept: 1,
            absent: 1
        }
    );
    let moved = sessions.load(&moving).unwrap().unwrap();
    assert_eq!(moved.tokens.access_token, "old-token");
    assert_eq!(
        sessions.load(&kept).unwrap().unwrap().tokens.access_token,
        "newer"
    );
    assert!(old.get(&moving).unwrap().is_none());
    assert!(old.get(&kept).unwrap().is_none());

    let reads = old.reads();
    let again = migrate_sessions(&old, &sessions, &accounts[..2], &dir.lock()).unwrap();
    assert_eq!(again.kept, 2);
    assert_eq!(old.reads(), reads, "nothing left to read");
}

/// A keychain whose errors quote the account, as the real one's can.
struct Quoting;

impl KeyStore for Quoting {
    fn get(&self, account: &str) -> Result<Option<Secret>, CliError> {
        Err(CliError::Keychain(format!("item `{account}` is damaged")))
    }

    fn set(&self, _: &str, _: &Secret) -> Result<(), CliError> {
        unreachable!()
    }

    fn delete(&self, _: &str) -> Result<bool, CliError> {
        unreachable!()
    }
}

#[test]
fn a_failed_move_names_no_patient() {
    let dir = Dir::new();
    let keys = Arc::new(MemoryKeys::new());
    let vault = dir.vault(&keys);
    let sessions = ProfileSessions::new(&vault, profile("adult1"));
    let err = migrate_sessions(
        &Quoting,
        &sessions,
        &[account("org-1", PATIENT)],
        &dir.lock(),
    )
    .unwrap_err();
    let text = err.to_string();
    assert!(!text.contains(PATIENT), "{text}");
    assert!(text.contains("smart/org-1/…"), "{text}");
}
