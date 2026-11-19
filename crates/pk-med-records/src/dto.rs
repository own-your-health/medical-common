//! The profile's DTOs: organizations, patient contexts, provenance and the
//! export summary. Every optional field is omitted when empty, so a field
//! added within `/v1` follows the family's additive carve-out.

use crate::timestamp::Timestamp;
use std::collections::BTreeMap;

use pk_cli_core::Paged;
use serde::{Deserialize, Serialize};

use crate::stream::{Finished, Header, Skip};
use pk_med_fhir::RawResource;

/// The profile's name, for `info`.
pub const PROFILE: &str = "health-records/v1";
pub const ORG_LIST_SCHEMA: &str = "org-list/v1";
pub const PATIENT_LIST_SCHEMA: &str = "patient-list/v1";
pub const PROVENANCE_SCHEMA: &str = "provenance/v1";
pub const EXPORT_SCHEMA: &str = "records-export/v1";

/// The `cli-info/v1` capability of a CLI whose login commands take
/// `--profile <NAME>` (DESIGN.md §1.1), `records export` among them. A
/// records store refuses to pass `--profile` to a provider without it,
/// which predates profiles. The list of profiles is cli-common's
/// `profile-list/v1` (`pk_cli_config::profile::ProfileList`).
pub const LOGIN_PROFILES: &str = "login-profiles";

/// The `cli-info/v1` capability of a CLI that reads records from files the
/// owner downloaded, with no login: `patients list` and `records export`
/// take `--files <DIR>` in place of `--profile` (DESIGN.md §1.1). A records
/// store passes `--files` only to a provider that lists it.
pub const RECORDS_FILES: &str = "records-files";

/// An organization a CLI can reach: a provider's FHIR server, or an
/// insurer's. Also the stream header's `organization`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    /// The CLI's own stable id for it (an Epic organization id, a slug).
    pub id: String,
    pub name: String,
    /// The FHIR base, normalized (`pk_med_fhir::normalize_base`). A source
    /// that is not a FHIR server (a member site) puts its site's base here.
    pub fhir_base: String,
    /// `epic`, `athenahealth`, `cerner`, `uhc`, ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    /// City and state, or a region, to tell same-named organizations apart.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// Other names it goes by (a brand, a former name), for `orgs search`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

/// A patient at a provider, or a member at an insurer, that a CLI holds a
/// sign-in for. Also the stream header's `patient`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatientContext {
    /// The FHIR `Patient.id` at the source.
    pub id: String,
    /// The [`Organization::id`] it is at.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub organization: String,
    /// The household's own name for the context (`--label`), never read from
    /// the record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// [`relationship::SELF`] or [`relationship::PROXY`] (a parent's access to
    /// a child's record).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship: Option<String>,
    /// [`kind::PATIENT`] at a provider, [`kind::MEMBER`] at an insurer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// A member's [`member_role::SUBSCRIBER`] or [`member_role::DEPENDENT`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_role: Option<String>,
    /// The signed-in user's `fhirUser` (`Patient/…`, `RelatedPerson/…`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fhir_user: Option<String>,
    /// The name the source shows, for `patients list` in the person's own
    /// terminal. Never in a fixture, a log or an error message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The CLI's keychain account for this sign-in (`smart/<org>/<patient>`),
    /// never the token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

/// `PatientContext.relationship` values.
pub mod relationship {
    pub const SELF: &str = "self";
    pub const PROXY: &str = "proxy";
}

/// `PatientContext.kind` values.
pub mod kind {
    pub const PATIENT: &str = "patient";
    pub const MEMBER: &str = "member";
}

/// `PatientContext.member_role` values.
pub mod member_role {
    pub const SUBSCRIBER: &str = "subscriber";
    pub const DEPENDENT: &str = "dependent";
}

/// `orgs list` and `orgs search`: `org-list/v1`.
pub fn org_list(items: Vec<Organization>) -> Paged<Organization> {
    Paged::new("org", items)
}

/// `patients list`: `patient-list/v1`.
pub fn patient_list(items: Vec<PatientContext>) -> Paged<PatientContext> {
    Paged::new("patient", items)
}

/// Where a filed record came from (`provenance/v1`), on every record a store
/// keeps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub source: ProvenanceSource,
    /// One of [`method`]'s values.
    pub method: String,
    /// The resource at the source, for a record read from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_resource: Option<SourceResource>,
    /// When the source was read (the stream's `pulled_at`), RFC 3339.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<String>,
    /// When the store filed it, RFC 3339.
    pub filed_at: String,
    /// The raw file in the records folder: the verbatim resource
    /// (`<Person>/Records/<source>/<Type>/<id>.json`), or the document as
    /// received.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_path: Option<String>,
    /// Who decided the person, date and type: one of [`classified_by`]'s
    /// values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classified_by: Option<String>,
    /// For a derived field: the document and the rule it came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<DerivedFrom>,
    /// The program and version that filed it (`medrec 0.3.0`).
    pub filed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceSource {
    /// One of [`source_kind`]'s values.
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fhir_base: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceResource {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedFrom {
    pub document_id: String,
    pub rule_id: String,
}

/// `Provenance.method` values: how the record was obtained.
pub mod method {
    pub const FHIR_API: &str = "fhir-api";
    /// Mapped from a member or patient site, or from files downloaded there.
    pub const PORTAL: &str = "portal";
    pub const SCAN: &str = "scan";
    pub const FILE: &str = "file";
    pub const MEDIA: &str = "media";
    pub const MANUAL: &str = "manual";
    pub const HUISHOUDEN: &str = "huishouden";
}

/// `ProvenanceSource.kind` values: what kind of source it is.
pub mod source_kind {
    pub const FHIR: &str = "fhir";
    pub const PORTAL: &str = "portal";
    pub const SCAN: &str = "scan";
    pub const DRIVE: &str = "drive";
    pub const EMAIL: &str = "email";
    pub const MEDIA: &str = "media";
    pub const MANUAL: &str = "manual";
    pub const HUISHOUDEN: &str = "huishouden";
}

/// `Provenance.classified_by` values.
pub mod classified_by {
    /// The store's own rules.
    pub const MEDREC: &str = "medrec";
    pub const ORGANIZE_SCANS: &str = "organize-scans";
    /// On the command line or in review.
    pub const HAND: &str = "hand";
}

impl Provenance {
    /// The provenance of one resource read from a stream: the source from the
    /// header (its `method`, `fhir-api` when absent), the resource's type, id,
    /// version and last update, and `fetched_at` from `pulled_at`. The store
    /// adds `raw_path` and `classified_by`.
    pub fn from_stream(
        header: &Header,
        resource: &RawResource,
        filed_at: impl Into<String>,
        filed_by: impl Into<String>,
    ) -> Self {
        let method = header.method().to_string();
        let kind = if method == method::FHIR_API {
            source_kind::FHIR.to_string()
        } else {
            method.clone()
        };
        Provenance {
            source: ProvenanceSource {
                kind,
                cli: Some(header.source.cli.clone()),
                cli_version: Some(header.source.version.clone()),
                organization_id: Some(header.organization.id.clone()),
                fhir_base: Some(header.organization.fhir_base.clone()),
            },
            method,
            source_resource: Some(SourceResource {
                resource_type: resource.resource_type.clone(),
                id: resource.id.clone(),
                version_id: resource.version_id().map(str::to_string),
                last_updated: resource.last_updated().map(str::to_string),
            }),
            fetched_at: Some(header.pulled_at.clone()),
            filed_at: filed_at.into(),
            raw_path: None,
            classified_by: None,
            derived_from: None,
            filed_by: filed_by.into(),
        }
    }

    /// When this version's content was current (DESIGN.md §1.4): the
    /// resource's `meta.lastUpdated` (`source_resource.last_updated`); for a
    /// resource from a FHIR API (`method` `fhir-api`) the server sent without
    /// one, when its stream was pulled (`fetched_at`). A mapped resource has
    /// no such fallback: its stream's pull time says when files were read,
    /// not when a statement was current. A time that does not read as an
    /// RFC 3339 instant ([`Timestamp::parse`]) counts as absent. `None` when
    /// no time is known.
    pub fn current_at(&self) -> Option<Timestamp> {
        let own = self
            .source_resource
            .as_ref()
            .and_then(|r| r.last_updated.as_deref())
            .and_then(Timestamp::parse);
        own.or_else(|| {
            (self.method == method::FHIR_API)
                .then(|| self.fetched_at.as_deref().and_then(Timestamp::parse))
                .flatten()
        })
    }

    /// Whether a store holding `held` replaces it with this version of the
    /// same resource (DESIGN.md §1.4), by [`current_at`](Self::current_at):
    ///
    /// | `held` | this | |
    /// |---|---|---|
    /// | known | known, later | replace |
    /// | known | known, the same or earlier | keep `held` |
    /// | known | unknown | keep `held` |
    /// | unknown | known or unknown | replace |
    ///
    /// A tie keeps `held`, so a re-import changes nothing. A version with no
    /// known time replaces only one with none either, as the last import
    /// did before versions were dated. A store asks it through
    /// [`version::decide`](crate::version::decide), which first sets apart a
    /// version that differs from `held` only in `meta.lastUpdated`.
    pub fn supersedes(&self, held: &Provenance) -> bool {
        match (self.current_at(), held.current_at()) {
            (Some(new), Some(old)) => new > old,
            (None, Some(_)) => false,
            (_, None) => true,
        }
    }
}

/// What `records export -o <DIR>` wrote (`records-export/v1`), on stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordsExport {
    pub schema: String,
    /// The stream file written (`<DIR>/records.ndjson`).
    pub path: String,
    pub bytes: u64,
    /// The trailer's `complete`.
    pub complete: bool,
    /// Resources written, by type.
    pub counts: BTreeMap<String, u64>,
    /// What was left out, and why.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped: Vec<Skip>,
}

impl RecordsExport {
    /// The summary of a stream written to `path`, from what
    /// [`crate::RecordsWriter::finish`] returned.
    pub fn new(path: impl Into<String>, finished: &Finished) -> Self {
        let trailer = &finished.trailer;
        RecordsExport {
            schema: EXPORT_SCHEMA.into(),
            path: path.into(),
            bytes: finished.bytes,
            complete: trailer.complete,
            counts: trailer.counts.clone(),
            skipped: trailer.skipped.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::Source;

    #[test]
    fn lists_carry_their_schemas() {
        assert_eq!(org_list(vec![]).schema, ORG_LIST_SCHEMA);
        assert_eq!(patient_list(vec![]).schema, PATIENT_LIST_SCHEMA);
    }

    #[test]
    fn empty_optionals_are_omitted() {
        let p = PatientContext {
            id: "pat-0001".into(),
            organization: String::new(),
            label: None,
            relationship: None,
            kind: None,
            member_role: None,
            fhir_user: None,
            name: None,
            session: None,
        };
        assert_eq!(serde_json::to_string(&p).unwrap(), r#"{"id":"pat-0001"}"#);
        let back: PatientContext = serde_json::from_str(r#"{"id":"pat-0001"}"#).unwrap();
        assert_eq!(back, p);
    }

    /// A filed version with the given `meta.lastUpdated` and `fetched_at`.
    fn version(last_updated: Option<&str>, fetched_at: Option<&str>) -> Provenance {
        Provenance {
            source: ProvenanceSource {
                kind: source_kind::PORTAL.into(),
                cli: Some("uhc".into()),
                cli_version: None,
                organization_id: Some("uhc".into()),
                fhir_base: None,
            },
            method: method::PORTAL.into(),
            source_resource: Some(SourceResource {
                resource_type: "ExplanationOfBenefit".into(),
                id: "eob-0001".into(),
                version_id: None,
                last_updated: last_updated.map(str::to_string),
            }),
            fetched_at: fetched_at.map(str::to_string),
            filed_at: "2026-03-05T00:00:00Z".into(),
            raw_path: None,
            classified_by: None,
            derived_from: None,
            filed_by: "medrec 0.0.0".into(),
        }
    }

    /// A FHIR API version with the given `meta.lastUpdated` and pull time.
    fn fhir_version(last_updated: Option<&str>, fetched_at: &str) -> Provenance {
        let mut p = version(last_updated, Some(fetched_at));
        p.method = method::FHIR_API.into();
        p.source.kind = source_kind::FHIR.into();
        p
    }

    #[test]
    fn a_version_is_current_at_its_last_update_or_a_fhir_pull() {
        let at = |t: &str| Timestamp::parse(t).unwrap();
        let pulled = "2026-03-04T10:00:00Z";
        let dated = version(Some("2026-03-01T00:00:00Z"), Some(pulled));
        assert_eq!(dated.current_at(), Some(at("2026-03-01T00:00:00Z")));
        // A FHIR resource the server sent undated was current when pulled.
        assert_eq!(fhir_version(None, pulled).current_at(), Some(at(pulled)));
        assert_eq!(
            fhir_version(Some("not a time"), pulled).current_at(),
            Some(at(pulled))
        );
        // A mapped one has no fallback: a --files pull time is when the
        // folder was read, not when the statement was current.
        assert_eq!(version(None, Some(pulled)).current_at(), None);
        assert_eq!(version(Some("2026-03-01"), Some(pulled)).current_at(), None);
        assert_eq!(version(None, None).current_at(), None);
    }

    #[test]
    fn a_version_supersedes_the_held_one_only_when_it_is_more_current() {
        // A site read on 2026-03-10, held; a statement issued 2026-03-01.
        let site = version(Some("2026-03-10T14:00:00Z"), Some("2026-03-10T14:00:00Z"));
        let statement = version(Some("2026-03-01T00:00:00Z"), Some("2026-03-12T09:00:00Z"));
        assert!(!statement.supersedes(&site));
        assert!(site.supersedes(&statement));
        // An undated statement, from a producer before the rule or with no
        // printed date, does not replace a dated site version, though it was
        // imported later.
        let undated = version(None, Some("2026-03-12T09:00:00Z"));
        assert!(!undated.supersedes(&site));
        // A dated version replaces an undated one filed before the rule, and
        // with neither dated the last import wins, as before.
        assert!(site.supersedes(&undated));
        assert!(undated.supersedes(&version(None, Some("2026-03-01T00:00:00Z"))));
        // The same instant in two offsets is a tie; a tie keeps the held.
        let same = version(Some("2026-03-10T09:00:00-05:00"), None);
        assert!(!same.supersedes(&site));
        assert!(!site.supersedes(&same));
        // Undated FHIR versions compare by when they were pulled.
        let january = fhir_version(None, "2026-01-05T00:00:00Z");
        let march = fhir_version(None, "2026-03-05T00:00:00Z");
        assert!(march.supersedes(&january));
        assert!(!january.supersedes(&march));
    }

    #[test]
    fn provenance_from_a_stream_names_the_source_and_resource() {
        let header = Header::new(
            Source::new("mychart", "0.1.0"),
            Organization {
                id: "org-1".into(),
                name: "Example Health".into(),
                fhir_base: "https://fhir.example.test/r4".into(),
                vendor: Some("epic".into()),
                location: None,
                aliases: vec![],
            },
            PatientContext {
                id: "pat-0001".into(),
                organization: "org-1".into(),
                label: None,
                relationship: Some(relationship::SELF.into()),
                kind: None,
                member_role: None,
                fhir_user: None,
                name: None,
                session: None,
            },
            "2020-01-03T00:00:00Z",
        );
        let raw = RawResource::parse(
            r#"{"resourceType":"Observation","id":"obs-0001","meta":{"versionId":"2"}}"#,
        )
        .unwrap();
        let p = Provenance::from_stream(&header, &raw, "2020-01-04T00:00:00Z", "medrec 0.0.0");
        assert_eq!(
            (p.method.as_str(), p.source.kind.as_str()),
            ("fhir-api", "fhir")
        );
        let source = p.source_resource.as_ref().unwrap();
        assert_eq!(
            (source.resource_type.as_str(), source.id.as_str()),
            ("Observation", "obs-0001")
        );
        assert_eq!(source.version_id.as_deref(), Some("2"));
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["source_resource"]["type"], "Observation");
        assert!(json.get("raw_path").is_none());

        let mut portal = header.clone();
        portal.method = Some(method::PORTAL.into());
        let p = Provenance::from_stream(&portal, &raw, "t", "medrec");
        assert_eq!(
            (p.method.as_str(), p.source.kind.as_str()),
            ("portal", "portal")
        );
    }
}
