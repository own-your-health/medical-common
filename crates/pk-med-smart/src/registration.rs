//! What a vendor's developer form asks for, so a CLI's `registration`
//! command prints it the same way as every other CLI (DESIGN.md §3.8), and
//! what a CLI says when no client is configured.
//!
//! A [`Registration`] is the CLI's own description of its app: the portal,
//! the guide in its repository, the launch and client type, PKCE, the
//! redirect URIs, the scopes, and every other field the form asks for. It
//! renders as text ([`Registration::text`]) and as `registration/v1` JSON
//! (serde). [`missing_client`] is the error a command gives when nothing is
//! configured to sign in with. [`ClientStatus`] is the `client` member that
//! `info` and `auth status` add ([`with_client`]).

use std::fmt::Write as _;

use pk_cli_core::{CliError, Reason};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scopes::ScopeSet;

/// The `schema` of a [`Registration`].
pub const SCHEMA: &str = "registration/v1";

/// Why a command could not sign in: no client is configured
/// ([`missing_client`]). Compare a [`CliError`]'s reason with it.
pub const NO_CLIENT: Reason = Reason::Provider("no-client");

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

/// What to enter in a vendor's developer form to register the CLI
/// (`registration/v1`). Build it with [`Registration::new`] and its builder
/// methods: it is `#[non_exhaustive]`, so a member a later release adds
/// comes with a default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Registration {
    /// [`SCHEMA`].
    pub schema: String,
    /// The CLI's binary name.
    pub cli: String,
    /// Who registers the app (`Epic on FHIR`, `athenahealth`).
    pub vendor: String,
    /// The developer portal, where the form is.
    pub portal: String,
    /// The step-by-step guide in the CLI's repository
    /// (`docs/registration.md`).
    pub doc: String,
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
    /// `portal`, guided by `doc`. `store` is the command that stores the
    /// result; it names placeholders (`<CLIENT_ID>`), never a value.
    pub fn new(
        cli: impl Into<String>,
        vendor: impl Into<String>,
        portal: impl Into<String>,
        doc: impl Into<String>,
        client_type: ClientType,
        store: impl Into<String>,
    ) -> Self {
        Registration {
            schema: SCHEMA.into(),
            cli: cli.into(),
            vendor: vendor.into(),
            portal: portal.into(),
            doc: doc.into(),
            launch: LaunchType::StandalonePatient,
            client_type,
            pkce: Pkce::S256,
            redirect_uris: Vec::new(),
            scopes: Vec::new(),
            urls: Vec::new(),
            fields: Vec::new(),
            store: store.into(),
            notes: Vec::new(),
        }
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

    /// Add a note.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// The human form: the portal and the guide, then each value under the
    /// label to enter it at, then the command that stores the result.
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
        let _ = writeln!(
            out,
            "Register {} with {}: {}",
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

/// The error a command gives when no client is configured to sign in with
/// (exit 2, reason [`NO_CLIENT`]). The message names the vendor and, for a
/// CLI with more than one registration, which one (`sandbox`); the remedy
/// names the guide, the `registration` command and the command that stores
/// the result. Neither names a client id.
pub fn missing_client(registration: &Registration, which: Option<&str>) -> CliError {
    let which = which
        .map(|w| format!(" for the {w} registration"))
        .unwrap_or_default();
    CliError::Usage(format!(
        "no {} client is configured{which}",
        registration.vendor
    ))
    .because(NO_CLIENT)
    .with_remedy(format!(
        "register one as {} describes, entering what `{} registration` prints, then run `{}`",
        registration.doc, registration.cli, registration.store
    ))
}

/// Whether a client is configured, and which: the `client` member `info`
/// and `auth status` add to their JSON ([`with_client`]), and a line of
/// their human output ([`ClientStatus::text`]).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClientStatus {
    pub configured: bool,
    /// Which registration, for a CLI with more than one (`production`,
    /// `sandbox`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_type: Option<ClientType>,
    /// Where it is kept: `config` or `vault`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stored_in: Option<String>,
    /// The client id, for a CLI that keeps it in its config. A CLI that keeps
    /// it in the vault leaves it out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
}

impl ClientStatus {
    /// Nothing is configured.
    pub fn none() -> Self {
        ClientStatus::default()
    }

    /// A configured client of `client_type`, kept in `stored_in`.
    pub fn configured(client_type: ClientType, stored_in: impl Into<String>) -> Self {
        ClientStatus {
            configured: true,
            client_type: Some(client_type),
            stored_in: Some(stored_in.into()),
            ..ClientStatus::default()
        }
    }

    /// Name the registration (`sandbox`).
    pub fn registration(mut self, which: impl Into<String>) -> Self {
        self.registration = Some(which.into());
        self
    }

    /// Show the client id, for a CLI that keeps it in its config.
    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        self.client_id = Some(id.into());
        self
    }

    /// One line for human output: `client: public, in config (sandbox)`, or
    /// that none is configured and where the guide is.
    pub fn text(&self, registration: &Registration) -> String {
        if !self.configured {
            return format!(
                "client: none configured; see {} and `{} registration`",
                registration.doc, registration.cli
            );
        }
        let mut out = String::from("client: ");
        out.push_str(match self.client_type {
            Some(ClientType::Public) => "public",
            Some(ClientType::ConfidentialSymmetric) => "confidential (secret)",
            Some(ClientType::ConfidentialAsymmetric) => "confidential (key)",
            None => "configured",
        });
        if let Some(id) = &self.client_id {
            let _ = write!(out, " {id}");
        }
        if let Some(stored) = &self.stored_in {
            let _ = write!(out, ", in {stored}");
        }
        if let Some(which) = &self.registration {
            let _ = write!(out, " ({which})");
        }
        out
    }
}

/// `dto` (`cli-info/v1`, `auth-status/v1`) as JSON with a `client` member
/// added. Readers that predate it ignore it.
pub fn with_client<T: Serialize>(dto: &T, client: &ClientStatus) -> Result<Value, CliError> {
    let mut value = serde_json::to_value(dto)
        .map_err(|e| CliError::Other(format!("serializing the output: {e}")))?;
    let Value::Object(map) = &mut value else {
        return Err(CliError::Other("the output is not a JSON object".into()));
    };
    let client = serde_json::to_value(client)
        .map_err(|e| CliError::Other(format!("serializing the client: {e}")))?;
    map.insert("client".into(), client);
    Ok(value)
}

#[cfg(test)]
mod tests;
