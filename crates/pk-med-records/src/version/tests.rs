use super::*;
use crate::dto::{method, source_kind, ProvenanceSource, SourceResource};

fn eob(text: &str) -> RawResource {
    RawResource::parse(text).unwrap()
}

/// A synthetic claim as a site shows it, stamped with when it was read.
fn claim(stamp: &str, total: &str) -> RawResource {
    eob(&format!(
        r#"{{"resourceType":"ExplanationOfBenefit","id":"eob-0001","meta":{{"lastUpdated":"{stamp}","source":"uhc:portal"}},"status":"active","total":[{{"amount":{{"value":{total},"currency":"USD"}}}}]}}"#
    ))
}

fn filed(how: &str, last_updated: Option<&str>, fetched_at: &str) -> Provenance {
    Provenance {
        source: ProvenanceSource {
            kind: if how == method::FHIR_API {
                source_kind::FHIR
            } else {
                source_kind::PORTAL
            }
            .into(),
            cli: Some("uhc".into()),
            cli_version: None,
            organization_id: Some("uhc".into()),
            fhir_base: None,
        },
        method: how.into(),
        source_resource: Some(SourceResource {
            resource_type: "ExplanationOfBenefit".into(),
            id: "eob-0001".into(),
            version_id: None,
            last_updated: last_updated.map(str::to_string),
        }),
        fetched_at: Some(fetched_at.into()),
        filed_at: "2026-03-20T00:00:00Z".into(),
        raw_path: None,
        classified_by: None,
        derived_from: None,
        filed_by: "medrec 0.0.0".into(),
    }
}

/// A site read at `at`, stamped with it (§1.4).
fn site(at: &str) -> Provenance {
    filed(method::PORTAL, Some(at), at)
}

const MARCH_10: &str = "2026-03-10T14:00:00Z";
const MARCH_15: &str = "2026-03-15T08:00:00Z";
const MARCH_15_READ: &str = "2026-03-15T09:30:00Z";

#[test]
fn a_stamp_alone_does_not_make_another_version() {
    let first = claim(MARCH_10, "12.50");
    let again = claim(MARCH_15, "12.50");
    assert_ne!(first.raw, again.raw);
    assert!(same_version(&first, &again));
    assert_eq!(content(&first), content(&again));
    assert!(!same_version(&first, &claim(MARCH_15, "13.50")));
}

#[test]
fn member_order_does_not_count_and_a_decimal_spelling_does() {
    let a = eob(
        r#"{"resourceType":"Observation","id":"o1","meta":{"source":"s","lastUpdated":"2026-03-10T14:00:00Z"},"valueQuantity":{"value":1.5}}"#,
    );
    let b = eob(
        r#"{"id":"o1","valueQuantity":{"value":1.5},"meta":{"lastUpdated":"2026-03-15T08:00:00Z","source":"s"},"resourceType":"Observation"}"#,
    );
    let c = eob(
        r#"{"resourceType":"Observation","id":"o1","meta":{"source":"s"},"valueQuantity":{"value":1.50}}"#,
    );
    assert!(same_version(&a, &b));
    assert!(!same_version(&a, &c), "1.5 and 1.50 differ in precision");
}

#[test]
fn a_meta_left_empty_is_no_meta_and_its_other_members_count() {
    let stamped = eob(
        r#"{"resourceType":"Coverage","id":"c1","meta":{"lastUpdated":"2026-03-10T14:00:00Z"},"status":"active"}"#,
    );
    let bare = eob(r#"{"resourceType":"Coverage","id":"c1","status":"active"}"#);
    let empty = eob(r#"{"resourceType":"Coverage","id":"c1","meta":{},"status":"active"}"#);
    let versioned =
        eob(r#"{"resourceType":"Coverage","id":"c1","meta":{"versionId":"2"},"status":"active"}"#);
    assert!(same_version(&stamped, &bare));
    assert!(same_version(&bare, &empty));
    assert!(!same_version(&bare, &versioned));
    assert_eq!(
        content(&stamped),
        r#"{"id":"c1","resourceType":"Coverage","status":"active"}"#
    );
}

#[test]
fn a_site_read_again_unchanged_refreshes_the_held_copy() {
    let held = claim(MARCH_10, "12.50");
    let again = claim(MARCH_15, "12.50");
    assert_eq!(
        decide(&held, &site(MARCH_10), &again, &site(MARCH_15)),
        Filing::Refreshed
    );
    // The same stamp again, as a re-import of one stream: nothing to file.
    assert_eq!(
        decide(&held, &site(MARCH_10), &held, &site(MARCH_10)),
        Filing::Unchanged
    );
    // An earlier read imported after a later one changes nothing.
    assert_eq!(
        decide(&again, &site(MARCH_15), &held, &site(MARCH_10)),
        Filing::Unchanged
    );
}

#[test]
fn other_content_replaces_unless_less_current() {
    let held = claim(MARCH_10, "12.50");
    let later = claim(MARCH_15, "13.50");
    assert_eq!(
        decide(&held, &site(MARCH_10), &later, &site(MARCH_15)),
        Filing::Replaces
    );
    assert_eq!(
        decide(&later, &site(MARCH_15), &held, &site(MARCH_10)),
        Filing::Older
    );
    // The same content at the same time is still nothing to file.
    assert_eq!(
        decide(
            &held,
            &site(MARCH_10),
            &claim(MARCH_10, "12.50"),
            &listed(MARCH_10, MARCH_15_READ)
        ),
        Filing::Unchanged
    );
    // A statement with no printed date never displaces a dated read.
    let undated =
        eob(r#"{"resourceType":"ExplanationOfBenefit","id":"eob-0001","status":"active"}"#);
    let statement = filed(method::PORTAL, None, "2026-03-20T09:00:00Z");
    assert_eq!(
        decide(&held, &site(MARCH_10), &undated, &statement),
        Filing::Older
    );
}

#[test]
fn an_undated_fhir_resource_pulled_again_refreshes_by_its_pull_time() {
    let line = eob(r#"{"resourceType":"ExplanationOfBenefit","id":"eob-0001","status":"active"}"#);
    let january = filed(method::FHIR_API, None, "2026-01-05T00:00:00Z");
    let march = filed(method::FHIR_API, None, "2026-03-05T00:00:00Z");
    assert_eq!(decide(&line, &january, &line, &march), Filing::Refreshed);
    assert_eq!(decide(&line, &march, &line, &january), Filing::Unchanged);
}

#[test]
fn a_refresh_keeps_what_the_store_decided() {
    let mut held = site(MARCH_10);
    held.raw_path = Some("Member/Records/uhc/ExplanationOfBenefit/eob-0001.json".into());
    held.classified_by = Some(crate::dto::classified_by::HAND.into());
    held.derived_from = Some(crate::dto::DerivedFrom {
        document_id: "doc-0001".into(),
        rule_id: "rule-0001".into(),
    });
    let mut incoming = site(MARCH_15);
    incoming.source.cli_version = Some("0.2.2".into());
    incoming.filed_at = "2026-03-21T00:00:00Z".into();
    incoming.filed_by = "medrec 0.0.1".into();
    let refreshed = held.refreshed_by(&incoming);
    assert_eq!(refreshed.source, incoming.source);
    assert_eq!(refreshed.method, incoming.method);
    assert_eq!(refreshed.source_resource, incoming.source_resource);
    assert_eq!(refreshed.fetched_at.as_deref(), Some(MARCH_15));
    assert_eq!(refreshed.filed_at, held.filed_at);
    assert_eq!(refreshed.filed_by, held.filed_by);
    assert_eq!(refreshed.raw_path, held.raw_path);
    assert_eq!(refreshed.classified_by, held.classified_by);
    assert_eq!(refreshed.derived_from, held.derived_from);
    assert!(
        !site(MARCH_15).supersedes(&refreshed),
        "the refresh is now current"
    );
}

/// A copy listed with `listed` and read at `read`, by the site.
fn listed(listed: &str, read: &str) -> Provenance {
    filed(method::PORTAL, Some(listed), read)
}

#[test]
fn a_later_read_by_the_same_route_breaks_a_tie() {
    // A report the site reissues under the date it lists, in another offset.
    let preliminary = claim(MARCH_10, "12.50");
    let final_ = claim("2026-03-10T09:00:00-05:00", "13.50");
    let first = listed(MARCH_10, MARCH_10);
    let then = listed("2026-03-10T09:00:00-05:00", MARCH_15_READ);
    assert_eq!(
        decide(&preliminary, &first, &final_, &then),
        Filing::Replaces
    );
    // Re-importing the earlier stream afterwards changes nothing.
    assert_eq!(decide(&final_, &then, &preliminary, &first), Filing::Older);
    // A read at the same moment, or one with no read time, cannot order them.
    assert_eq!(
        decide(&preliminary, &first, &final_, &listed(MARCH_10, MARCH_10)),
        Filing::Older
    );
    let mut unread = listed(MARCH_10, MARCH_15_READ);
    unread.fetched_at = None;
    assert_eq!(
        decide(&preliminary, &first, &final_, &unread),
        Filing::Older
    );
}

#[test]
fn two_routes_at_the_same_time_keep_the_held_copy() {
    // One statement, listed on the site and downloaded, dated the same day.
    let site_copy = claim(MARCH_10, "12.50");
    let statement_copy = eob(&site_copy.raw.replace("uhc:portal", "uhc:eob-pdf"));
    let site_read = listed(MARCH_10, MARCH_10);
    let folder_read = listed(MARCH_10, MARCH_15_READ);
    assert!(!same_version(&site_copy, &statement_copy));
    assert_eq!(
        decide(&site_copy, &site_read, &statement_copy, &folder_read),
        Filing::Older
    );
    assert_eq!(
        decide(&statement_copy, &folder_read, &site_copy, &site_read),
        Filing::Older
    );
}
