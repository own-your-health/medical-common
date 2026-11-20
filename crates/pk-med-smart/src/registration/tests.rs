use super::*;
use crate::scopes::ScopeVersion;

const GUIDE: &str = "https://example.com/examplecli/docs/registration.md";

/// A synthetic CLI's sandbox registration: placeholders and example hosts
/// only.
fn example() -> Registration {
    Registration::new(
        "examplecli",
        "Example Vendor",
        "https://developer.example.com/apps",
        GUIDE,
        ClientType::Public,
        "examplecli config set client-id <CLIENT_ID> --sandbox",
    )
    .registration("sandbox")
    .command("examplecli registration --sandbox")
    .redirect_uri("http://127.0.0.1:53682/callback")
    .redirect_uri("https://example.github.io/examplecli/callback")
    .scopes(&ScopeSet::standalone_patient().read_all(ScopeVersion::V2))
    .url("Privacy policy", "https://example.com/privacy")
    .url_with_note("Terms of service", "https://example.com/terms", "any page")
    .field("Application audience", "Patients")
    .field_with_note("Requires persistent access", "Yes", "so a sign-in renews")
    .selection(
        "Incoming APIs",
        ["Patient.Read (R4)", "Observation.Search (R4)"],
    )
    .selection_with_note("Outgoing APIs", ["None"], "the CLI only reads")
    .note("The portal refuses an http redirect other than 127.0.0.1.")
}

/// The same CLI with one registration.
fn single() -> Registration {
    Registration::new(
        "examplecli",
        "Example Vendor",
        "https://developer.example.com/apps",
        GUIDE,
        ClientType::ConfidentialSymmetric,
        "examplecli auth set-credential --stdin",
    )
}

#[test]
fn the_text_puts_each_value_under_the_label_to_enter_it_at() {
    let text = example().text();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        "Register examplecli with Example Vendor (sandbox): https://developer.example.com/apps"
    );
    assert_eq!(lines[1], format!("Step by step: {GUIDE}"));
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
        row("Terms of service", "https://example.com/terms"),
        row("", "(any page)"),
        row("Application audience", "Patients"),
        row("Requires persistent access", "Yes"),
        row("", "(so a sign-in renews)"),
        "Incoming APIs".to_string(),
        "  Patient.Read (R4)".to_string(),
        "  Observation.Search (R4)".to_string(),
        "(the CLI only reads)".to_string(),
        "Then store what the portal gives back: examplecli config set client-id <CLIENT_ID> --sandbox"
            .to_string(),
        "- The portal refuses an http redirect other than 127.0.0.1.".to_string(),
    ] {
        assert!(lines.contains(&expected.as_str()), "{expected:?} in\n{text}");
    }
    let at = |l: &str| lines.iter().position(|x| *x == l).unwrap();
    assert!(at("URLs") < at("Other fields"));
    assert!(at("Other fields") < at("Incoming APIs"));
    assert!(at("Incoming APIs") < at("Outgoing APIs"));
    assert!(single()
        .text()
        .starts_with("Register examplecli with Example Vendor: "));
}

#[test]
fn the_json_is_registration_v2_in_the_spec_spelling() {
    let value = serde_json::to_value(example()).unwrap();
    assert_eq!(value["schema"], "registration/v2");
    assert_eq!(value["registration"], "sandbox");
    assert_eq!(value["command"], "examplecli registration --sandbox");
    assert_eq!(value["doc"], GUIDE);
    assert_eq!(value["launch"], "standalone-patient");
    assert_eq!(value["client_type"], "public");
    assert_eq!(value["pkce"], "S256");
    assert_eq!(value["redirect_uris"].as_array().unwrap().len(), 2);
    assert_eq!(value["scopes"][0], "openid");
    assert_eq!(value["urls"][1]["note"], "any page");
    assert_eq!(value["fields"][1]["note"], "so a sign-in renews");
    assert!(value["fields"][0].get("note").is_none());
    assert_eq!(value["selections"][0]["label"], "Incoming APIs");
    assert_eq!(
        value["selections"][0]["items"][1],
        "Observation.Search (R4)"
    );
    assert_eq!(value["selections"][1]["note"], "the CLI only reads");
    let back: Registration = serde_json::from_value(value).unwrap();
    assert_eq!(back, example());
    let bare = serde_json::to_value(single()).unwrap();
    assert_eq!(bare["client_type"], "confidential-symmetric");
    assert_eq!(bare["command"], "examplecli registration");
    for absent in ["registration", "urls", "fields", "selections", "notes"] {
        assert!(bare.get(absent).is_none(), "{absent}");
    }
}

#[test]
fn a_missing_client_names_the_guide_and_that_registrations_commands() {
    let err = missing_client(&example());
    assert_eq!(err.exit_code(), 2);
    assert_eq!(err.reason(), Some(NO_CLIENT));
    assert_eq!(NO_CLIENT, Reason::Provider("no_client"));
    assert_eq!(
        err.message(),
        "no Example Vendor client is configured for the sandbox registration"
    );
    assert_eq!(
        err.remedy().unwrap(),
        format!(
            "register one by following {GUIDE}, entering what `examplecli registration --sandbox` \
             prints, then run `examplecli config set client-id <CLIENT_ID> --sandbox`"
        )
    );
    let one = missing_client(&single());
    assert_eq!(one.message(), "no Example Vendor client is configured");
    assert!(one.remedy().unwrap().contains("`examplecli registration`"));
}

#[test]
fn a_status_line_names_its_registration_configured_or_not() {
    let reg = example();
    assert_eq!(
        ClientStatus::of(&reg, None).text(&reg),
        format!(
            "client (sandbox): none configured; see {GUIDE} and `examplecli registration --sandbox`"
        )
    );
    let status = ClientStatus::of(&reg, Some(StoredIn::Config))
        .client_id_last_four("0000-example-client-ab12");
    assert_eq!(status.registration.as_deref(), Some("sandbox"));
    assert_eq!(
        status.text(&reg),
        "client (sandbox): public, ending ab12, in config"
    );
    let one = single();
    assert_eq!(
        ClientStatus::of(&one, Some(StoredIn::Vault)).text(&one),
        "client: confidential (secret), in vault"
    );
    assert_eq!(
        ClientStatus::none().text(&one),
        format!("client: none configured; see {GUIDE} and `examplecli registration`")
    );
}

#[test]
fn a_client_id_shows_four_characters_at_most_and_none_when_short() {
    let shown = |id: &str| {
        ClientStatus::none()
            .client_id_last_four(id)
            .client_id_ends_with
    };
    assert_eq!(shown("000000000000").as_deref(), Some("0000"));
    assert_eq!(shown("0000-0000-wxyz").as_deref(), Some("wxyz"));
    assert_eq!(shown("00000000000"), None);
    assert_eq!(shown(""), None);
    assert_eq!(shown("ééééééééééé·αβγδ").as_deref(), Some("αβγδ"));
}

#[test]
fn clients_are_one_entry_per_registration() {
    let reg = example();
    let production = single().registration("production");
    let rows = [
        ClientStatus::of(&production, Some(StoredIn::Vault)),
        ClientStatus::of(&reg, None),
    ];
    let info = serde_json::json!({"schema": "cli-info/v1", "name": "examplecli"});
    let out = with_clients(&info, &rows).unwrap();
    assert_eq!(out["schema"], "cli-info/v1");
    assert_eq!(
        out["clients"],
        serde_json::json!([
            {"registration": "production", "configured": true,
             "client_type": "confidential-symmetric", "stored_in": "vault"},
            {"registration": "sandbox", "configured": false}
        ])
    );
    let lone = with_clients(&info, &[ClientStatus::of(&single(), None)]).unwrap();
    assert_eq!(lone["clients"], serde_json::json!([{"configured": false}]));
}

#[test]
fn a_clients_member_that_cannot_be_read_is_refused() {
    let info = serde_json::json!({"schema": "auth-status/v1"});
    let refused = |dto: &Value, rows: &[ClientStatus]| {
        let err = with_clients(dto, rows).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        err.message().to_string()
    };
    assert!(refused(&info, &[]).contains("no client status"));
    let unnamed = [
        ClientStatus::none(),
        ClientStatus::none().registration("sandbox"),
    ];
    assert!(refused(&info, &unnamed).contains("its own registration"));
    let twice = [
        ClientStatus::none().registration("sandbox"),
        ClientStatus::none().registration("sandbox"),
    ];
    assert!(refused(&info, &twice).contains("its own registration"));
    let taken = serde_json::json!({"schema": "auth-status/v1", "clients": []});
    assert!(refused(&taken, &[ClientStatus::none()]).contains("already has a `clients` member"));
    assert!(with_clients(&"text", &[ClientStatus::none()]).is_err());
}

#[test]
fn a_guide_is_an_https_url_with_a_host() {
    for good in [GUIDE, "HTTPS://example.com/g", "https://h"] {
        assert!(is_https_url(good), "{good}");
    }
    for bad in [
        "docs/registration.md",
        "http://example.com/g",
        "https://",
        "https:///path",
        "https://?q",
        "ftp://example.com",
        "",
    ] {
        assert!(!is_https_url(bad), "{bad}");
    }
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "must be the guide's absolute https URL")]
fn a_repository_path_for_the_guide_fails_a_debug_build() {
    let _ = Registration::new(
        "examplecli",
        "Example Vendor",
        "https://developer.example.com/apps",
        "docs/registration.md",
        ClientType::Public,
        "examplecli config set client-id <CLIENT_ID>",
    );
}
