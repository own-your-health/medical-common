//! Sessions kept per profile, in the CLI's vault.
//!
//! A profile is one login (`adult1`), named by the owner and kept in the
//! CLI's local config (`pk_cli_config::profile`). Its sessions live in the
//! CLI's [`Vault`] (`pk_cli_auth::vault`), one entry per organization and
//! patient, named `<profile>/<organization>/<patient>`. The whole CLI then
//! reads one keychain item, the vault's key, so a new or upgraded binary
//! asks macOS once rather than once per sign-in.
//!
//! [`ProfileSessions`] is a [`SessionStore`]: the CLI hands it to a
//! [`Renewer`](crate::Renewer) and stores a sign-in through it, with the same
//! [`account`](crate::account) strings as before. [`migrate_sessions`] moves
//! sessions kept the old way, one keychain item each, into a profile.
//!
//! A CLI's other secrets share the vault: name them with a leading `_`
//! (`_client-secret`), which no profile name has, so a profile's listing
//! and `profiles remove` never touch them.

use pk_cli_auth::refresh_lock::RefreshLock;
use pk_cli_auth::vault::{KeyStore, Migration, Vault};
use pk_cli_config::profile::ProfileName;
use pk_cli_core::CliError;
use pk_cli_secrets::Secret;

use crate::session::{decode_session, encode_session, without_account, SessionStore, SmartSession};

/// The prefix of an [`account`](crate::account); a profile's entry puts the
/// profile's name in its place.
const ACCOUNT_PREFIX: &str = "smart/";

/// One profile's sessions in the CLI's vault. See the [module docs](self).
pub struct ProfileSessions<'v> {
    vault: &'v Vault,
    profile: ProfileName,
}

impl<'v> ProfileSessions<'v> {
    pub fn new(vault: &'v Vault, profile: ProfileName) -> Self {
        Self { vault, profile }
    }

    pub fn profile(&self) -> &ProfileName {
        &self.profile
    }

    /// The vault entry of `account`: `<profile>/<organization>/<patient>`.
    pub fn entry_name(&self, account: &str) -> Result<String, CliError> {
        let rest = account.strip_prefix(ACCOUNT_PREFIX).ok_or_else(|| {
            CliError::Other("a session account must be one `pk_med_smart::account` made".into())
        })?;
        Ok(format!("{}/{rest}", self.profile))
    }

    /// The accounts this profile keeps a session for, sorted. Reads the
    /// vault's key when the vault has any entry.
    pub fn accounts(&self) -> Result<Vec<String>, CliError> {
        let prefix = format!("{}/", self.profile);
        Ok(self
            .vault
            .names()?
            .into_iter()
            .filter_map(|name| {
                name.strip_prefix(&prefix)
                    .map(|rest| format!("{ACCOUNT_PREFIX}{rest}"))
            })
            .collect())
    }

    /// Delete every session this profile keeps, for `profiles remove`;
    /// how many there were.
    pub fn delete_all(&self) -> Result<usize, CliError> {
        let mut deleted = 0;
        for account in self.accounts()? {
            if SessionStore::delete(self, &account)? {
                deleted += 1;
            }
        }
        Ok(deleted)
    }
}

/// The vault's errors name an entry's file, never its name or a value, so
/// they pass through as they are.
impl SessionStore for ProfileSessions<'_> {
    fn load(&self, account: &str) -> Result<Option<SmartSession>, CliError> {
        let Some(text) = self.vault.get(&self.entry_name(account)?)? else {
            return Ok(None);
        };
        decode_session(text.expose()).map(Some)
    }

    fn save(&self, session: &SmartSession) -> Result<(), CliError> {
        let name = self.entry_name(&session.account())?;
        self.vault.put(&name, &encode_session(session)?)
    }

    fn delete(&self, account: &str) -> Result<bool, CliError> {
        self.vault.delete(&self.entry_name(account)?)
    }
}

/// Move the sessions at `accounts` from `from` (the keychain, one item each:
/// `CredentialStore::for_binary(bin)`) into `to`, a profile's sessions,
/// holding `lock` (the CLI's refresh lock) throughout, so no renewal writes
/// either side meanwhile. Nothing goes over the network and nothing needs
/// the patient's approval. An item's text is copied as it is.
///
/// Per account:
/// - only the old item: it is written to the profile, read back and
///   compared, and only then deleted (`moved`);
/// - both: the newer sign-in wins, by when its tokens were obtained. An old
///   item newer than the profile's copy (signed in again after a downgrade)
///   replaces it the same way (`moved`); otherwise the profile's copy is
///   kept and the item deleted (`kept`). An item that does not read loses;
/// - only the profile's copy, or neither: nothing to do (`kept`, `absent`).
///
/// A failure leaves the old item in place, so a second run picks up where
/// the first stopped. A run with nothing left to move only looks up items
/// that are gone, which raises no dialog. An error names no patient.
///
/// Each old item read by a newly built binary costs one macOS dialog, once.
/// Delete test and sandbox items beforehand (`security
/// delete-generic-password -s piekstra.<bin> -a <account>`, which raises
/// none) rather than move them. A downgrade past the move finds no sign-in.
/// Do not call this while holding `lock`: it is not re-entrant.
pub fn migrate_sessions(
    from: &dyn KeyStore,
    to: &ProfileSessions<'_>,
    accounts: &[String],
    lock: &RefreshLock,
) -> Result<Migration, CliError> {
    lock.hold(|| {
        let mut done = Migration::default();
        for account in accounts {
            migrate_one(from, to, account, &mut done).map_err(|e| without_account(e, account))?;
        }
        Ok(done)
    })
}

fn migrate_one(
    from: &dyn KeyStore,
    to: &ProfileSessions<'_>,
    account: &str,
    done: &mut Migration,
) -> Result<(), CliError> {
    let name = to.entry_name(account)?;
    let held = to.vault.get(&name)?;
    let Some(item) = from.get(account)? else {
        if held.is_some() {
            done.kept += 1;
        } else {
            done.absent += 1;
        }
        return Ok(());
    };
    if let Some(held) = held {
        let obtained = |text: &Secret| {
            decode_session(text.expose())
                .ok()
                .map(|s| s.tokens.obtained_at)
        };
        let item_is_newer = match (obtained(&item), obtained(&held)) {
            (Some(item), Some(held)) => item > held,
            (Some(_), None) => true,
            (None, _) => false,
        };
        if !item_is_newer {
            from.delete(account)?;
            done.kept += 1;
            return Ok(());
        }
    }
    to.vault.put(&name, &item)?;
    let same = to
        .vault
        .get(&name)?
        .is_some_and(|back| back.expose() == item.expose());
    if !same {
        return Err(CliError::Other(
            "a session written to the vault did not read back the same; the keychain item is kept"
                .into(),
        ));
    }
    from.delete(account)?;
    done.moved += 1;
    Ok(())
}
