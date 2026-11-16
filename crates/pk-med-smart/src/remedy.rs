//! What to do next after a pk-med-smart error, kept apart from what went
//! wrong.
//!
//! Every error here carries its remedy through `CliError::with_remedy`, never
//! in its message. A CLI replaces it with its own exact command
//! (``err.with_remedy("run `kp auth login --org …`")``) instead of printing
//! both, and tells the remedies apart by `CliError::remedy`. [`SIGN_IN_AGAIN`]
//! and [`TRY_AGAIN`] are spelled as `pk_med_fhir::outcome`'s, so one match
//! serves the errors of both crates.

/// Nothing is stored for this organization and patient (exit 3,
/// `Reason::NoCredential`).
pub const SIGN_IN: &str = "sign in";
/// The sign-in can no longer be used: it lapsed with no way to renew it, or
/// it was refused (exit 3, `Reason::CredentialRefused`).
pub const SIGN_IN_AGAIN: &str = "sign in again";
/// The stored sign-in does not read (`CliError::Keychain`).
pub const CLEAR_AND_SIGN_IN_AGAIN: &str = "clear the stored sign-in and sign in again";
/// The stored sign-in was written by a newer release (`CliError::Keychain`).
pub const UPDATE: &str = "update this CLI";
/// The server answered a 429 or a 5xx and said nothing more (exit 5).
pub const TRY_AGAIN: &str = "try again later";

#[cfg(test)]
mod tests {
    #[test]
    fn the_shared_remedies_are_spelled_as_pk_med_fhirs() {
        assert_eq!(super::SIGN_IN_AGAIN, pk_med_fhir::outcome::SIGN_IN_AGAIN);
        assert_eq!(super::TRY_AGAIN, pk_med_fhir::outcome::TRY_AGAIN);
    }
}
