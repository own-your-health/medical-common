//! What a literal reference (`Reference.reference`) names.

use std::fmt;

use crate::client::{has_dot_segment, is_id, is_type};
use crate::normalize_base;

/// The resource a literal reference names: `Type/id` or
/// `Type/id/_history/vid`, relative to the server's base or after an
/// absolute `http(s)` base (`https://fhir.example.org/r4/Patient/p1`).
///
/// [`ReferenceTarget::parse`] reads one. A local reference (`#org1`), a
/// `urn:uuid:` or `urn:oid:` one, a conditional one (`Patient?identifier=…`),
/// and anything whose type, id or version FHIR does not allow are `None`.
/// [`ReferenceTarget::read`] tells a refused id or version apart (`Err`), for
/// a caller that reports such a reference rather than drop it; everything
/// else `parse` refuses, a disallowed type included, is `Ok(None)` there.
/// The id and version pass [`is_id`], so `resource_type` and `id` go into
/// [`FhirClient::read`](crate::FhirClient::read) as they are. Whether a
/// reference on another base is followed, and which types are read, is the
/// caller's: [`ReferenceTarget::is_under`] tells the first.
///
/// ```
/// use pk_med_fhir::ReferenceTarget;
/// let t = ReferenceTarget::parse("https://fhir.example.org/r4/DocumentReference/d1/_history/2").unwrap();
/// assert_eq!((t.resource_type, t.id, t.version), ("DocumentReference", "d1", Some("2")));
/// assert!(t.is_under("https://FHIR.example.org/r4/"));
/// assert!(ReferenceTarget::parse("#org1").is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReferenceTarget<'a> {
    /// The base of an absolute reference as written, without the `/` before
    /// the type; `None` for a relative reference.
    pub base: Option<&'a str>,
    pub resource_type: &'a str,
    pub id: &'a str,
    /// The `_history` version, when the reference names one.
    pub version: Option<&'a str>,
}

impl<'a> ReferenceTarget<'a> {
    /// Read a literal reference, as written: no surrounding whitespace, no
    /// trailing `/`. `None` for anything else, a refused id included
    /// ([`Self::read`] tells that case apart).
    pub fn parse(reference: &'a str) -> Option<Self> {
        Self::read(reference).ok().flatten()
    }

    /// As [`Self::parse`], with a reference shaped as one (a resource type,
    /// on no base or an absolute `http(s)` one) whose id or version
    /// [`is_id`] refuses kept apart: `Err`, with the type and the id as
    /// written, so a caller can report it. `Ok(None)` is not a literal
    /// reference at all (`#org1`, `urn:…`, `Patient?…`).
    pub fn read(reference: &'a str) -> Result<Option<Self>, RefusedReference<'a>> {
        if reference.contains(['?', '#']) {
            return Ok(None);
        }
        let Some(target) = Self::split(reference) else {
            return Ok(None);
        };
        if !is_type(target.resource_type) || target.base.is_some_and(|b| !is_absolute_base(b)) {
            return Ok(None);
        }
        if !is_id(target.id) || !target.version.is_none_or(is_id) {
            return Err(RefusedReference {
                base: target.base,
                resource_type: target.resource_type,
                id: target.id,
                version: target.version,
            });
        }
        Ok(Some(target))
    }

    /// `[base/]Type/id[/_history/vid]` by its slashes, checking nothing.
    fn split(reference: &'a str) -> Option<Self> {
        let (head, last) = reference.rsplit_once('/')?;
        let (head, id, version) = match head.rsplit_once('/') {
            Some((resource, "_history")) => {
                let (head, id) = resource.rsplit_once('/')?;
                (head, id, Some(last))
            }
            _ => (head, last, None),
        };
        let (base, resource_type) = match head.rsplit_once('/') {
            Some((base, resource_type)) => (Some(base), resource_type),
            None => (None, head),
        };
        Some(ReferenceTarget {
            base,
            resource_type,
            id,
            version,
        })
    }

    /// Whether the reference names a resource on `base`: a relative one
    /// always does, an absolute one when its base is `base` in
    /// [`normalize_base`]'s spelling.
    pub fn is_under(&self, base: &str) -> bool {
        is_under(self.base, base)
    }
}

fn is_under(own: Option<&str>, base: &str) -> bool {
    own.is_none_or(|b| normalize_base(b) == normalize_base(base))
}

/// A reference shaped as `Type/id` whose id or version [`is_id`] refuses
/// ([`ReferenceTarget::read`]). Its `Display` names the type and the id's
/// length; its `Debug` also shows the base (a server URL) and whether a
/// version is present. Neither shows the id or the version: an error is
/// logged, and a patient's id can be an identifier. The fields hold both for
/// a caller that needs them.
#[derive(Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RefusedReference<'a> {
    /// The base of an absolute reference as written, as
    /// [`ReferenceTarget::base`]; `None` for a relative reference.
    pub base: Option<&'a str>,
    pub resource_type: &'a str,
    /// The id as written.
    pub id: &'a str,
    /// The `_history` version as written, when there is one.
    pub version: Option<&'a str>,
}

impl RefusedReference<'_> {
    /// Whether the reference names a resource on `base`, as
    /// [`ReferenceTarget::is_under`].
    pub fn is_under(&self, base: &str) -> bool {
        is_under(self.base, base)
    }
}

impl fmt::Display for RefusedReference<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if is_id(self.id) {
            write!(
                f,
                "a reference to a {} names a version FHIR does not allow",
                self.resource_type
            )
        } else {
            write!(
                f,
                "a reference to a {} names an id FHIR does not allow ({} characters; ids are 1 \
                 to {} of A-Z a-z 0-9 - ., not only dots)",
                self.resource_type,
                self.id.chars().count(),
                crate::MAX_ID_LEN
            )
        }
    }
}

impl fmt::Debug for RefusedReference<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RefusedReference")
            .field("base", &self.base)
            .field("resource_type", &self.resource_type)
            .field("id_chars", &self.id.chars().count())
            .field("has_version", &self.version.is_some())
            .finish()
    }
}

impl std::error::Error for RefusedReference<'_> {}

/// `http(s)://<host>[/<segment>…]`: printable ASCII, no empty, `.` or `..`
/// segment (spelled out or percent-encoded).
fn is_absolute_base(base: &str) -> bool {
    let Some((scheme, rest)) = base.split_once("://") else {
        return false;
    };
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    (scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http"))
        && !authority.is_empty()
        && base.chars().all(|c| c.is_ascii_graphic())
        && !path.split('/').skip(1).any(str::is_empty)
        && !has_dot_segment(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_ID_LEN;

    fn parts(reference: &str) -> Option<(Option<&str>, &str, &str, Option<&str>)> {
        ReferenceTarget::parse(reference).map(|t| (t.base, t.resource_type, t.id, t.version))
    }

    #[test]
    fn relative_absolute_and_versioned_references_are_read() {
        let base = Some("https://fhir.example.test/r4");
        for (reference, expected) in [
            ("Patient/p1", (None, "Patient", "p1", None)),
            ("Patient/p1/_history/2", (None, "Patient", "p1", Some("2"))),
            (
                "https://fhir.example.test/r4/DocumentReference/d.1-a",
                (base, "DocumentReference", "d.1-a", None),
            ),
            (
                "https://fhir.example.test/r4/Binary/b1/_history/v-3",
                (base, "Binary", "b1", Some("v-3")),
            ),
            (
                "HTTP://FHIR.example.test:8443/Patient/p1",
                (Some("HTTP://FHIR.example.test:8443"), "Patient", "p1", None),
            ),
        ] {
            assert_eq!(parts(reference), Some(expected), "{reference}");
        }
        let longest = format!("Patient/{}", "e".repeat(MAX_ID_LEN));
        assert_eq!(parts(&longest).unwrap().2.len(), MAX_ID_LEN);
    }

    #[test]
    fn what_is_not_a_literal_reference_is_none() {
        let over = format!("Patient/{}", "e".repeat(MAX_ID_LEN + 1));
        for reference in [
            "",
            "Patient",
            "Patient/",
            "/Patient/p1",
            "Patient/p1/",
            "folder/Patient/p1",
            "patient/p1",
            "Pa-tient/p1",
            "Patient/p 1",
            "Patient/p_1",
            "Patient/.",
            "Patient/..",
            "Patient/p1/extra",
            "Patient/_history/2",
            "Patient/p1/_history/",
            "Patient/p1/_history/v_1",
            "Patient/p1/_history/2/x",
            "#org1",
            "Patient/p1#x",
            "urn:uuid:5e7a2c1e-0000-4000-8000-000000000001",
            "urn:oid:2.16.840.1.113883",
            "Patient?identifier=x",
            "Patient/p1?x=1",
            "ftp://fhir.example.test/Patient/p1",
            "https:///Patient/p1",
            "https://fhir.example.test//Patient/p1",
            "https://fhir.example.test/r4/../Patient/p1",
            "https://fhir.example.test/r4/%2E%2e/Patient/p1",
            "https://fhir.example.test/r 4/Patient/p1",
            " Patient/p1",
            over.as_str(),
        ] {
            assert_eq!(parts(reference), None, "{reference:?}");
        }
    }

    #[test]
    fn read_keeps_a_refused_id_apart_from_what_is_not_a_reference() {
        let over = "e".repeat(MAX_ID_LEN + 1);
        let over_ref = format!("https://fhir.example.test/r4/Patient/{over}");
        for (reference, resource_type, id, version) in [
            ("Patient/p_1", "Patient", "p_1", None),
            ("Patient/..", "Patient", "..", None),
            ("Patient/", "Patient", "", None),
            ("Observation/o 1", "Observation", "o 1", None),
            (over_ref.as_str(), "Patient", over.as_str(), None),
            ("Patient/p1/_history/v_1", "Patient", "p1", Some("v_1")),
        ] {
            let refused = ReferenceTarget::read(reference).unwrap_err();
            assert_eq!(
                (refused.resource_type, refused.id, refused.version),
                (resource_type, id, version),
                "{reference}"
            );
            assert_eq!(ReferenceTarget::parse(reference), None, "{reference}");
        }
        for reference in [
            "#org1",
            "urn:uuid:5e7a2c1e-0000-4000-8000-000000000001",
            "Patient?identifier=x",
            "folder/Patient/p_1",
            "ftp://fhir.example.test/Patient/p_1",
            "patient/p_1",
            "Patient",
        ] {
            assert_eq!(ReferenceTarget::read(reference), Ok(None), "{reference}");
        }
        assert_eq!(
            ReferenceTarget::read("Patient/p1").unwrap().map(|t| t.id),
            Some("p1")
        );
    }

    #[test]
    fn a_refused_reference_keeps_its_base() {
        let base = "https://fhir.example.test/r4";
        let relative = ReferenceTarget::read("Patient/p_1").unwrap_err();
        assert_eq!(relative.base, None);
        assert!(relative.is_under(base));
        let own = ReferenceTarget::read("HTTPS://FHIR.example.test/r4/Patient/p_1").unwrap_err();
        assert_eq!(own.base, Some("HTTPS://FHIR.example.test/r4"));
        assert!(own.is_under(&format!("{base}/")));
        let other = ReferenceTarget::read("https://other.example.test/r4/Patient/p_1").unwrap_err();
        assert!(!other.is_under(base));
    }

    #[test]
    fn a_refused_reference_names_its_type_and_not_its_id() {
        let long = format!("DocumentReference/{}", "x".repeat(300));
        let text = ReferenceTarget::read(&long).unwrap_err().to_string();
        assert!(text.contains("DocumentReference"), "{text}");
        assert!(text.contains("300 characters"), "{text}");
        assert!(!text.contains("xxxx"), "{text}");
        let text = ReferenceTarget::read("Patient/p1/_history/v_1")
            .unwrap_err()
            .to_string();
        assert!(text.contains("version"), "{text}");
        for reference in ["Patient/secret_1", "Patient/p1/_history/secret_2"] {
            let debug = format!("{:?}", ReferenceTarget::read(reference));
            assert!(debug.contains("Patient"), "{debug}");
            assert!(!debug.contains("secret"), "{debug}");
        }
    }

    #[test]
    fn a_reference_is_under_its_own_base_in_any_spelling() {
        let base = "https://fhir.example.test/r4";
        let relative = ReferenceTarget::parse("Patient/p1").unwrap();
        assert!(relative.is_under(base));
        let absolute = ReferenceTarget::parse("https://FHIR.Example.test/r4/Patient/p1").unwrap();
        assert!(absolute.is_under(base));
        assert!(absolute.is_under("https://fhir.example.test/r4/"));
        assert!(!absolute.is_under("https://fhir.example.test"));
        assert!(!absolute.is_under("https://fhir.example.test/r4/other"));
        assert!(!absolute.is_under("https://other.example.test/r4"));
        assert!(!absolute.is_under("http://fhir.example.test/r4"));
    }

    #[test]
    fn a_reference_field_reads_its_target() {
        let r = crate::Reference {
            reference: Some("Organization/o1".into()),
            ..Default::default()
        };
        assert_eq!(r.target().map(|t| t.id), Some("o1"));
        assert_eq!(crate::Reference::default().target(), None);
    }
}
