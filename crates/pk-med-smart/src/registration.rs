//! What a vendor's developer form asks for, so a CLI's `registration`
//! command prints it the same way as every other CLI (DESIGN.md §3.8), and
//! what a CLI says when no client is configured.
//!
//! A [`Registration`] is the CLI's own description of one registration of
//! its app: the portal, the guide, the launch and client type, PKCE, the
//! redirect URIs, the scopes, and every other field the form asks for. A CLI
//! with a sandbox and a production registration builds one of each. It
//! renders as text ([`Registration::text`]) and as `registration/v1` JSON
//! (serde). [`missing_client`] is the error a command gives when nothing is
//! configured to sign in with. [`ClientStatus`] says whether one is, per
//! registration: the `clients` member that `info` and `auth status` add
//! ([`with_clients`]).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use pk_cli_core::{CliError, Reason};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scopes::ScopeSet;

/// The `schema` of a [`Registration`].
pub const SCHEMA: &str = "registration/v1";

/// Why a command could not sign in: no client is configured
/// ([`missing_client`]). Compare a [`CliError`]'s reason with it; `--json`
/// prints it as `no_client` from cli-common v0.33.4.
pub const NO_CLIENT: Reason = Reason::Provider("no_client");

/// An id this short shows no characters in [`ClientStatus`].
const SHORTEST_ID_SHOWN: usize = 12;

/// The kind of launch the app is registered for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum LaunchType {
    /// A standalone patient launch: the patient starts the app and picks
    /// themself, or a person they hold access for.
    StandalonePatient,
}

/// How the client authenticates at the token endpoint, in SMART's terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ClientType {
    /// No secret: PKCE alone protects the code.
    Public,
    /// A client secret, kept in the CLI's vault.
    ConfidentialSymmetric,
    /// A key pair: the CLI signs a JWT with its private key (§3.4).
    ConfidentialAsymmetric,
}

/// How the code is bound to the sign-in that asked for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Pkce {
    /// `code_challenge_method=S256`, sent on every sign-in.
    S256,
}

/// One field of the vendor's form: its label as the portal prints it, the
/// value to enter, and why, where that is not plain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormField {
    pub label: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A list the form asks the registrant to pick from, and the items to pick
/// (Epic's APIs): its label as the portal prints it, and each item as the
/// portal names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub label: String,
    pub items: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// What to enter in a vendor's developer form for one registration of the
/// CLI (`registration/v1`). Build it with [`Registration::new`] and its
/// builder methods: it is `#[non_exhaustive]`, so a member a later release
/// adds comes with a default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Registration {
    /// [`SCHEMA`].
    pub schema: String,
    /// The CLI's binary name.
    pub cli: String,
    /// Who registers the app (`Epic on FHIR`, `athenahealth`).
    pub vendor: String,
    /// Which registration this is (`sandbox`, `production`), for a CLI with
    /// more than one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration: Option<String>,
    /// The developer portal, where the form is.
    pub portal: String,
    /// The step-by-step guide, as an absolute `https` URL, so that someone
    /// who installed a release binary and has no repository can open it.
    pub doc: String,
    /// The command that prints this registration: `<cli> registration` with
    /// whatever selects it (`--env preview`).
    pub command: String,
    pub launch: LaunchType,
    pub client_type: ClientType,
    pub pkce: Pkce,
    /// Every redirect URI to register, in the order the CLI tries them.
    pub redirect_uris: Vec<String>,
    /// Every scope to ask for.
    pub scopes: Vec<String>,
    /// The URLs the form asks for (a privacy policy, a launch URL), in its
    /// order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urls: Vec<FormField>,
    /// Every other field the form asks for, in its order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FormField>,
    /// The lists the form asks the registrant to pick from, in its order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selections: Vec<Selection>,
    /// The command that stores what the portal gives back
    /// (`mychart config set client-id <CLIENT_ID>`).
    pub store: String,
    /// What the portal refuses and the way around it, review times, and the
    /// like: one sentence each.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Registration {
    /// A standalone patient launch with PKCE S256, for `cli` at `vendor`'s
    /// `portal`, guided by `doc` (an absolute `https` URL). `store` is the
    /// command that stores the result; it names placeholders
    /// (`<CLIENT_ID>`), never a value. The command that prints it is
    /// `<cli> registration` until [`Self::command`] says otherwise.
    pub fn new(
        cli: impl Into<String>,
        vendor: impl Into<String>,
        portal: impl Into<String>,
        doc: impl Into<String>,
        client_type: ClientType,
        store: impl Into<String>,
    ) -> Self {
        let cli = cli.into();
        Registration {
            schema: SCHEMA.into(),
            command: format!("{cli} registration"),
            cli,
            vendor: vendor.into(),
            registration: None,
            portal: portal.into(),
            doc: doc.into(),
            launch: LaunchType::StandalonePatient,
            client_type,
            pkce: Pkce::S256,
            redirect_uris: Vec::new(),
            scopes: Vec::new(),
            urls: Vec::new(),
            fields: Vec::new(),
            selections: Vec::new(),
            store: store.into(),
            notes: Vec::new(),
        }
    }

    /// Name the registration (`sandbox`), for a CLI with more than one.
    pub fn registration(mut self, which: impl Into<String>) -> Self {
        self.registration = Some(which.into());
        self
    }

    /// The command that prints this registration, when it is not
    /// `<cli> registration` (`athena registration --env preview`).
    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.command = command.into();
        self
    }

    /// Add a redirect URI.
    pub fn redirect_uri(mut self, uri: impl Into<String>) -> Self {
        self.redirect_uris.push(uri.into());
        self
    }

    /// The scopes the CLI asks for, as it asks for them.
    pub fn scopes(mut self, scopes: &ScopeSet) -> Self {
        self.scopes = scopes.iter().map(str::to_string).collect();
        self
    }

    /// Add a URL the form asks for.
    pub fn url(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.urls.push(FormField::new(label, value, None));
        self
    }

    /// Add a URL the form asks for, with why that value.
    pub fn url_with_note(
        mut self,
        label: impl Into<String>,
        value: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        self.urls
            .push(FormField::new(label, value, Some(note.into())));
        self
    }

    /// Add another field the form asks for.
    pub fn field(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.push(FormField::new(label, value, None));
        self
    }

    /// Add another field, with why that value.
    pub fn field_with_note(
        mut self,
        label: impl Into<String>,
        value: impl Into<String>,
        note: impl Into<String>,
    ) -> Self {
        self.fields
            .push(FormField::new(label, value, Some(note.into())));
        self
    }

    /// Add a list to pick from, and the items to pick in it.
    pub fn selection<I: Into<String>>(
        mut self,
        label: impl Into<String>,
        items: impl IntoIterator<Item = I>,
    ) -> Self {
        self.selections.push(Selection::new(label, items, None));
        self
    }

    /// Add a list to pick from, with why those items.
    pub fn selection_with_note<I: Into<String>>(
        mut self,
        label: impl Into<String>,
        items: impl IntoIterator<Item = I>,
        note: impl Into<String>,
    ) -> Self {
        self.selections
            .push(Selection::new(label, items, Some(note.into())));
        self
    }

    /// Add a note.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// The human form: the portal and the guide, then each value under the
    /// label to enter it at, the lists to pick from, then the command that
    /// stores the result.
    pub fn text(&self) -> String {
        let mut core = vec![
            ("Launch type", vec![launch_text(self.launch).to_string()]),
            (
                "Client type",
                vec![client_text(self.client_type).to_string()],
            ),
            ("PKCE", vec![pkce_text(self.pkce).to_string()]),
            ("Redirect URIs", self.redirect_uris.clone()),
            ("Scopes", self.scopes.clone()),
        ];
        core.retain(|(_, values)| !values.is_empty());
        let width = core
            .iter()
            .map(|(label, _)| label.len())
            .chain(self.urls.iter().chain(&self.fields).map(|f| f.label.len()))
            .max()
            .unwrap_or(0)
            + 2;
        let mut out = String::new();
        let which = self.which();
        let _ = writeln!(
            out,
            "Register {} with {}{which}: {}",
            self.cli, self.vendor, self.portal
        );
        let _ = writeln!(out, "Step by step: {}", self.doc);
        out.push('\n');
        for (label, values) in &core {
            write_values(&mut out, label, values, width);
        }
        for (heading, group) in [("URLs", &self.urls), ("Other fields", &self.fields)] {
            if group.is_empty() {
                continue;
            }
            let _ = writeln!(out, "\n{heading}");
            for field in group {
                write_values(
                    &mut out,
                    &field.label,
                    std::slice::from_ref(&field.value),
                    width,
                );
                if let Some(note) = &field.note {
                    let _ = writeln!(out, "{:width$}({note})", "");
                }
            }
        }
        for selection in &self.selections {
            let _ = writeln!(out, "\n{}", selection.label);
            if let Some(note) = &selection.note {
                let _ = writeln!(out, "({note})");
            }
            for item in &selection.items {
                let _ = writeln!(out, "  {item}");
            }
        }
        let _ = writeln!(
            out,
            "\nThen store what the portal gives back: {}",
            self.store
        );
        if !self.notes.is_empty() {
            out.push_str("\nNotes\n");
            for note in &self.notes {
                let _ = writeln!(out, "- {note}");
            }
        }
        out
    }

    fn which(&self) -> String {
        self.registration
            .as_deref()
            .map(|w| format!(" ({w})"))
            .unwrap_or_default()
    }
}

impl FormField {
    fn new(label: impl Into<String>, value: impl Into<String>, note: Option<String>) -> Self {
        FormField {
            label: label.into(),
            value: value.into(),
            note,
        }
    }
}

impl Selection {
    fn new<I: Into<String>>(
        label: impl Into<String>,
        items: impl IntoIterator<Item = I>,
        note: Option<String>,
    ) -> Self {
        Selection {
            label: label.into(),
            items: items.into_iter().map(Into::into).collect(),
            note,
        }
    }
}

fn write_values(out: &mut String, label: &str, values: &[String], width: usize) {
    for (i, value) in values.iter().enumerate() {
        let shown = if i == 0 { label } else { "" };
        let _ = writeln!(out, "{shown:width$}{value}");
    }
}

fn launch_text(launch: LaunchType) -> &'static str {
    match launch {
        LaunchType::StandalonePatient => "standalone patient launch",
    }
}

fn client_text(client: ClientType) -> &'static str {
    match client {
        ClientType::Public => "public (no client secret)",
        ClientType::ConfidentialSymmetric => "confidential, with a client secret",
        ClientType::ConfidentialAsymmetric => "confidential, with a public key (JWT)",
    }
}

fn pkce_text(pkce: Pkce) -> &'static str {
    match pkce {
        Pkce::S256 => "required, S256",
    }
}

/// The error a command gives when no client is configured for
/// `registration` (exit 2, reason [`NO_CLIENT`]). The message names the
/// vendor, and the registration where it has a name; the remedy names the
/// guide, the command that prints the registration and the command that
/// stores the result. Neither names a client id.
pub fn missing_client(registration: &Registration) -> CliError {
    let which = registration
        .registration
        .as_deref()
        .map(|w| format!(" for the {w} registration"))
        .unwrap_or_default();
    CliError::Usage(format!(
        "no {} client is configured{which}",
        registration.vendor
    ))
    .because(NO_CLIENT)
    .with_remedy(format!(
        "register one by following {}, entering what `{}` prints, then run `{}`",
        registration.doc, registration.command, registration.store
    ))
}

/// Where a configured client is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum StoredIn {
    /// The CLI's config file: a client id, which is not a secret.
    Config,
    /// The CLI's vault (`pk_cli_auth::vault`): a secret or a key with it.
    Vault,
}

impl StoredIn {
    fn text(self) -> &'static str {
        match self {
            StoredIn::Config => "config",
            StoredIn::Vault => "vault",
        }
    }
}

/// Whether a client is configured for one registration, and which: an entry
/// of the `clients` member `info` and `auth status` add to their JSON
/// ([`with_clients`]), and a line of their human output
/// ([`ClientStatus::text`]).
///
/// It never carries a client id whole. A vendor's terms may keep the id
/// the registrant's alone (athenahealth's "Keys"), and `info` output gets
/// pasted into bug reports, so at most its last four characters are shown
/// ([`ClientStatus::client_id_last_four`]).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClientStatus {
    /// Which registration, for a CLI with more than one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration: Option<String>,
    pub configured: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_type: Option<ClientType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stored_in: Option<StoredIn>,
    /// The client id's last four characters, to tell two ids apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id_ends_with: Option<String>,
}

impl ClientStatus {
    /// Nothing is configured.
    pub fn none() -> Self {
        ClientStatus::default()
    }

    /// A configured client of `client_type`, kept in `stored_in`.
    pub fn configured(client_type: ClientType, stored_in: StoredIn) -> Self {
        ClientStatus {
            configured: true,
            client_type: Some(client_type),
            stored_in: Some(stored_in),
            ..ClientStatus::default()
        }
    }

    /// The status of `registration`: configured, of its client type, when
    /// `stored_in` says where; not configured when it is `None`. It names
    /// the registration as `registration` does.
    pub fn of(registration: &Registration, stored_in: Option<StoredIn>) -> Self {
        let status = match stored_in {
            Some(stored_in) => ClientStatus::configured(registration.client_type, stored_in),
            None => ClientStatus::none(),
        };
        ClientStatus {
            registration: registration.registration.clone(),
            ..status
        }
    }

    /// Name the registration (`sandbox`).
    pub fn registration(mut self, which: impl Into<String>) -> Self {
        self.registration = Some(which.into());
        self
    }

    /// Show the last four characters of `id`, for a CLI that reads the id
    /// without a secret (from its config). An id shorter than 12 characters
    /// shows none, since four would be a third of it or more.
    pub fn client_id_last_four(mut self, id: &str) -> Self {
        let chars: Vec<char> = id.chars().collect();
        self.client_id_ends_with =
            (chars.len() >= SHORTEST_ID_SHOWN).then(|| chars[chars.len() - 4..].iter().collect());
        self
    }

    /// One line for human output: `client (sandbox): public, ending ab12, in
    /// config`, or that none is configured, with the guide and the command
    /// that prints the registration.
    pub fn text(&self, registration: &Registration) -> String {
        let which = self
            .registration
            .as_deref()
            .map(|w| format!(" ({w})"))
            .unwrap_or_else(|| registration.which());
        if !self.configured {
            return format!(
                "client{which}: none configured; see {} and `{}`",
                registration.doc, registration.command
            );
        }
        let mut out = format!("client{which}: ");
        out.push_str(match self.client_type {
            Some(ClientType::Public) => "public",
            Some(ClientType::ConfidentialSymmetric) => "confidential (secret)",
            Some(ClientType::ConfidentialAsymmetric) => "confidential (key)",
            None => "configured",
        });
        if let Some(end) = &self.client_id_ends_with {
            let _ = write!(out, ", ending {end}");
        }
        if let Some(stored) = self.stored_in {
            let _ = write!(out, ", in {}", stored.text());
        }
        out
    }
}

/// `dto` (`cli-info/v1`, `auth-status/v1`) as JSON with a `clients` member
/// added: one [`ClientStatus`] per registration the CLI has. Readers that
/// predate it ignore it.
///
/// `clients` is medical-common's member, not cli-common's: neither schema
/// defines it (DESIGN.md §3.8, cli-common#89). Refused (exit 1), as a bug in
/// the CLI rather than something to print:
/// - a `dto` that already has a `clients` member, so a cli-common release
///   that adds its own fails a CLI's `info --json` test instead of being
///   replaced on the wire;
/// - no entries;
/// - more than one entry where an entry names no registration, or two name
///   the same one.
pub fn with_clients<T: Serialize>(dto: &T, clients: &[ClientStatus]) -> Result<Value, CliError> {
    if clients.is_empty() {
        return Err(CliError::Other("no client status to report".into()));
    }
    if clients.len() > 1 {
        let names: BTreeSet<_> = clients
            .iter()
            .filter_map(|c| c.registration.as_deref())
            .collect();
        if names.len() != clients.len() {
            return Err(CliError::Other(
                "each of several client statuses must name its own registration".into(),
            ));
        }
    }
    let mut value = serde_json::to_value(dto)
        .map_err(|e| CliError::Other(format!("serializing the output: {e}")))?;
    let Value::Object(map) = &mut value else {
        return Err(CliError::Other("the output is not a JSON object".into()));
    };
    if map.contains_key("clients") {
        return Err(CliError::Other(
            "the output already has a `clients` member; refusing to replace it".into(),
        ));
    }
    let clients = serde_json::to_value(clients)
        .map_err(|e| CliError::Other(format!("serializing the clients: {e}")))?;
    map.insert("clients".into(), clients);
    Ok(value)
}

#[cfg(test)]
mod tests;
