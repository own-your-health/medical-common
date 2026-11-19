//! Whether two versions of a resource are the same, and how a store files an
//! incoming one (DESIGN.md §1.4).
//!
//! A producer stamps a resource mapped from a site with the time it read the
//! page, so reading the site again changes the line even when nothing on the
//! page did. Versions whose content matches in everything but
//! `meta.lastUpdated` are one version: [`decide`] refreshes the held copy
//! instead of filing another.

use std::collections::BTreeMap;

use serde_json::value::RawValue;

use crate::dto::Provenance;
use crate::RawResource;

/// How a store files an incoming version of a resource it holds, by
/// [`decide`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filing {
    /// The same content, not more current: file nothing.
    Unchanged,
    /// The same content, more current (a site read again, a FHIR resource
    /// pulled again): rewrite the held copy in place with the incoming text,
    /// and its provenance with [`Provenance::refreshed_by`], which keeps
    /// what the store decided. No version is added.
    Refreshed,
    /// Other content, more current or as current: the incoming version
    /// becomes current. At the same time the last import wins, as when
    /// neither has a time: a document reissued under the date it lists, a
    /// newer release's mapping. The store may keep the held one as an
    /// earlier version.
    Replaces,
    /// Other content, less current, or with no known time against a held
    /// version with one: the held version stays current. The store may keep
    /// the incoming one as an earlier version.
    Older,
}

/// How a store holding `held`, filed with `held_provenance`, files
/// `incoming`: by whether their [`content`] is the same, then by
/// [`Provenance::supersedes`]. Other content at the same time replaces
/// `held`, because the times cannot order the two.
pub fn decide(
    held: &RawResource,
    held_provenance: &Provenance,
    incoming: &RawResource,
    incoming_provenance: &Provenance,
) -> Filing {
    let later = incoming_provenance.supersedes(held_provenance);
    // Neither supersedes the other only when both times are known and equal.
    let tie = !later && !held_provenance.supersedes(incoming_provenance);
    match (same_version(held, incoming), later || tie) {
        (true, _) if later => Filing::Refreshed,
        (true, _) => Filing::Unchanged,
        (false, true) => Filing::Replaces,
        (false, false) => Filing::Older,
    }
}

/// Whether `a` and `b` are the same version: their [`content`] is the same.
pub fn same_version(a: &RawResource, b: &RawResource) -> bool {
    content(a) == content(b)
}

/// What a version is compared by: the resource's JSON with
/// `meta.lastUpdated` left out, and a `meta` left empty by that dropped. Its
/// members and `meta`'s are written in key order, so their order does not
/// count. Every other value is kept as written, so a decimal's spelling
/// (`1.5`, `1.50`) does. A store may name a copy by a digest of it. Text that
/// is not a JSON object is kept whole.
pub fn content(resource: &RawResource) -> String {
    content_of(&resource.raw).unwrap_or_else(|| resource.raw.clone())
}

fn content_of(text: &str) -> Option<String> {
    let members: BTreeMap<String, &RawValue> = serde_json::from_str(text).ok()?;
    let meta = match members.get("meta") {
        Some(meta) => {
            let mut inner: BTreeMap<String, &RawValue> = serde_json::from_str(meta.get()).ok()?;
            inner.remove("lastUpdated");
            (!inner.is_empty()).then(|| object(inner.iter().map(|(k, v)| (k.as_str(), v.get()))))
        }
        None => None,
    };
    let rest = members
        .iter()
        .filter(|(key, _)| key.as_str() != "meta")
        .map(|(k, v)| (k.as_str(), v.get()));
    let mut out: BTreeMap<&str, &str> = rest.collect();
    if let Some(meta) = &meta {
        out.insert("meta", meta);
    }
    Some(object(out))
}

/// A JSON object from members already in key order, each value as written.
fn object<'a>(members: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut out = String::from("{");
    for (i, (key, value)) in members.into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serde_json::Value::String(key.to_string()).to_string());
        out.push(':');
        out.push_str(value);
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests;
