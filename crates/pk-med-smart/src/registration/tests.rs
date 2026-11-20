use super::*;
use crate::scopes::ScopeVersion;

/// A synthetic CLI's registration: placeholders and example hosts only.
fn example() -> Registration {
    Registration::new(
        "examplecli",
        "Example Vendor",
        "https://developer.example.com/apps",
        "docs/registration.md",
        ClientType::Public,
        "examplecli config set client-id <CLIENT_ID>",
    )
    .redirect_uri("http://127.0.0.1:53682/callback")
    .redirect_uri("https://example.github.io/examplecli/callback")
    .scopes(&ScopeSet::standalone_patient().read_all(ScopeVersion::V2))
    .url("Privacy policy", "https://example.com/privacy")
    .field("Application audience", "Patients")
    .field_with_note("Requires persistent access", "Yes", "so a sign-in renews")
    .note("The portal refuses an http redirect other than 127.0.0.1.")
}

#[test]
fn the_text_puts_each_value_under_the_label_to_enter_it_at() {
    let text = example().text();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        "Register examplecli with Example Vendor: https://developer.example.com/apps"
    );
    assert_eq!(lines[1], "Step by step: docs/registration.md");
    let width = "Requires persistent access".len() + 2;
    let row = |label: &str, value: &str| format!("{label:width$}{value}");
    for expected in [
        row("Launch type", "standalone patient launch"),
        row("Client type", "public (no client secret)"),
        row("PKCE", "required, S256"),
        row("Redirect URIs", "http://127.0.0.1:53682/callback"),
        row("", "https://example.github.io/examplecli/callback"),
        row("Scopes", "openid"),
        row("", "offline_access"),
        row("Privacy policy", "https://example.com/privacy"),
        row("Application audience", "Patients"),
        row("Requires persistent access", "Yes"),
        row("", "(so a sign-in renews)"),
        "Then store what the portal gives back: examplecli config set client-id <CLIENT_ID>"
            .to_string(),
        "- The portal refuses an http redirect other than 127.0.0.1.".to_string(),
    ] {
        assert!(
            lines.contains(&expected.as_str()),
            "{expected:?} in\n{text}"
        );
    }
    let urls = lines.iter().position(|l| *l == "URLs").unwrap();
    let other = lines.iter().position(|l| *l == "Other fields").unwrap();
    assert!(urls < other);
}

#[test]
fn the_json_is_registration_v1_in_the_spec_spelling() {
    let value = serde_json::to_value(example()).unwrap();
    assert_eq!(value["schema"], "registration/v1");
    assert_eq!(value["launch"], "standalone-patient");
    assert_eq!(value["client_type"], "public");
    assert_eq!(value["pkce"], "S256");
    assert_eq!(value["redirect_uris"].as_array().unwrap().len(), 2);
    assert_eq!(value["scopes"][0], "openid");
    assert_eq!(value["fields"][1]["note"], "so a sign-in renews");
    assert!(value["fields"][0].get("note").is_none());
    let back: Registration = serde_json::from_value(value).unwrap();
    assert_eq!(back, example());
    let bare = Registration::new(
        "x",
        "V",
        "https://p.example.com",
        "d",
        ClientType::ConfidentialAsymmetric,
        "s",
    );
    let bare = serde_json::to_value(bare).unwrap();
    assert_eq!(bare["client_type"], "confidential-asymmetric");
    for absent in ["urls", "fields", "notes"] {
        assert!(bare.get(absent).is_none(), "{absent}");
    }
}

#[test]
fn a_missing_client_names_the_guide_and_the_commands() {
    let err = missing_client(&example(), Some("sandbox"));
    assert_eq!(err.exit_code(), 2);
    assert_eq!(err.reason(), Some(NO_CLIENT));
    assert_eq!(
        err.message(),
        "no Example Vendor client is configured for the sandbox registration"
    );
    let remedy = err.remedy().unwrap();
    for part in [
        "docs/registration.md",
        "`examplecli registration`",
        "`examplecli config set client-id <CLIENT_ID>`",
    ] {
        assert!(remedy.contains(part), "{part} in {remedy}");
    }
    assert_eq!(
        missing_client(&example(), None).message(),
        "no Example Vendor client is configured"
    );
}

#[test]
fn the_client_status_is_a_line_and_a_member() {
    let reg = example();
    assert_eq!(
        ClientStatus::none().text(&reg),
        "client: none configured; see docs/registration.md and `examplecli registration`"
    );
    let status = ClientStatus::configured(ClientType::Public, "config")
        .registration("sandbox")
        .client_id("example-client");
    assert_eq!(
        status.text(&reg),
        "client: public example-client, in config (sandbox)"
    );
    let vaulted = ClientStatus::configured(ClientType::ConfidentialSymmetric, "vault");
    assert_eq!(
        vaulted.text(&reg),
        "client: confidential (secret), in vault"
    );

    let info = serde_json::json!({"schema": "cli-info/v1", "name": "examplecli"});
    let out = with_client(&info, &vaulted).unwrap();
    assert_eq!(out["schema"], "cli-info/v1");
    assert_eq!(out["client"]["configured"], true);
    assert_eq!(out["client"]["client_type"], "confidential-symmetric");
    assert!(out["client"].get("client_id").is_none());
    assert_eq!(
        with_client(&ClientStatus::none(), &ClientStatus::none()).unwrap()["client"]["configured"],
        false
    );
    assert!(with_client(&"text", &vaulted).is_err());
}
