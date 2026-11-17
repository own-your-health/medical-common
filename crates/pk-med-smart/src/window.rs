//! The browser window a profile's sign-in opens in (feature `browser`).
//!
//! One Chrome profile per login: [`SignInWindow`] runs Chrome in the
//! profile's kept browser profile (`pk_cli_browser::Launch::profile_name`,
//! `browser-profile-<profile>` under the CLI's data directory), so one
//! login's cookies never sign in another, and a provider that remembers the
//! device remembers it per login. Pages the provider needs signed in first
//! open as tabs before the sign-in's own address: Kaiser Permanente's Epic
//! organizations answer their authorize page only inside a session from
//! kp.org's sign-on on the same domain, so mychart opens that first. The window
//! is closed by dropping it, which asks Chrome to quit over its debug port
//! so the cookies it just received are written, and the next sign-in in
//! that profile finds them.
//!
//! Remove the browser profile with `pk_cli_browser::forget_named_profile(bin,
//! profile)` when the profile is removed.

use pk_cli_browser::{Browser, Launch};
use pk_cli_config::profile::ProfileName;
use pk_cli_core::CliError;

use crate::launch::SmartSignIn;

/// A Chrome window in one profile's browser profile. Dropping it closes
/// Chrome. `Debug` shows no address.
#[derive(Debug)]
pub struct SignInWindow {
    _browser: Browser,
}

impl SignInWindow {
    /// Open `sign_in`'s address in `profile`'s browser profile, after a tab
    /// for each of `before`. Each address must be `https`, or `http` on
    /// `127.0.0.1` or `localhost`; anything else is refused before Chrome
    /// starts.
    pub fn open(
        bin: &str,
        profile: &ProfileName,
        sign_in: &SmartSignIn,
        before: &[&str],
    ) -> Result<Self, CliError> {
        Self::open_at(bin, profile, sign_in.url(), before)
    }

    /// [`open`](Self::open) at a start page of the CLI's own (a loopback
    /// page that sends the browser on to the authorize address).
    pub fn open_at(
        bin: &str,
        profile: &ProfileName,
        start: &str,
        before: &[&str],
    ) -> Result<Self, CliError> {
        Self::launch(bin, profile, start, before, |launch| launch)
    }

    /// [`open_at`](Self::open_at), with the launch adjusted first: a test's
    /// Chrome and profile root.
    pub(crate) fn launch(
        bin: &str,
        profile: &ProfileName,
        start: &str,
        before: &[&str],
        adjust: impl FnOnce(Launch) -> Launch,
    ) -> Result<Self, CliError> {
        let mut launch = Launch::new(bin).profile_name(profile.as_str());
        for url in tabs(start, before) {
            launch = launch.url(url);
        }
        let browser = adjust(launch).spawn().map_err(CliError::from)?;
        Ok(Self { _browser: browser })
    }
}

/// The tabs in the order they open: `before`, then the sign-in.
fn tabs<'a>(start: &'a str, before: &[&'a str]) -> Vec<&'a str> {
    before
        .iter()
        .copied()
        .chain(std::iter::once(start))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pages_before_open_first() {
        assert_eq!(
            tabs(
                "https://fhir.example.test/authorize?x=1",
                &["https://portal.example.test/sign-on"]
            ),
            [
                "https://portal.example.test/sign-on",
                "https://fhir.example.test/authorize?x=1"
            ]
        );
        assert_eq!(tabs("http://127.0.0.1:9/", &[]), ["http://127.0.0.1:9/"]);
    }

    #[test]
    fn a_bad_page_is_refused_before_chrome_starts() {
        let profile = ProfileName::parse("adult1").unwrap();
        let root = std::env::temp_dir().join(format!("pk-med-window-{}", std::process::id()));
        for bad in ["http://portal.example.test/", "--remote-allow-origins=*"] {
            let err = SignInWindow::launch(
                "pk-med-window-test",
                &profile,
                "https://fhir.example.test/authorize",
                &[bad],
                |l| {
                    l.chrome("/nonexistent/chrome")
                        .profile_root(&root)
                        .ctrl_c(false)
                        .quiet(true)
                },
            )
            .unwrap_err();
            assert!(err.to_string().contains("start URL"), "{err}");
        }
        assert!(!root.exists(), "nothing was created");
    }
}
