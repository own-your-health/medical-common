# Changelog

## Unreleased

- DESIGN.md §1.3: a CLI that maps a non-FHIR source derives
  `PatientContext.id` (from the caremark-cli session).
  - The id comes from an identifier every sign-in that sees the person
    shares. One that repeats across accounts (a person code) is combined
    with the account's.
  - It is a SHA-256 digest of length-prefixed identifiers, never a source
    identifier as written.
  - Where the source offers no shared identifier, the CLI derives the id per
    sign-in and says so in its docs. Two patients for one person can be
    merged by hand; one patient for two people cannot be cleanly split.
  - A change to the derivation is breaking: it splits every held patient,
    so the CLI's CHANGELOG names it and documents both derivations.
- DESIGN.md §1.4: uhc-cli's site `meta.source` is `uhc:member-site`, not
  `uhc:portal`.

## v0.6.6 — 2026-11-19

Pins cli-common v0.33.2, as v0.6.5 does.

- `pk-med-records`: `version::decide` now breaks a tie. A version with
  other content at the same time as the held one returns `Replaces` when
  both came by the same route (the same `meta.source`, or neither has one)
  and it was read later (`fetched_at`). Before, it was `Older`, and the
  store kept the held copy for good in three cases:
  - a document a site reissues under the date it lists (labcorp-cli: a
    report whose results come in under one `reportDate`);
  - a built resource whose latest record changed without changing its
    date;
  - a newer release's mapping of the same statement.

  A tie is still `Older` when the incoming version was read earlier or at
  the same time, when either read time is unknown, or when the two came by
  different routes. So re-importing an older stream changes nothing, and a
  statement listed on the site and the same statement downloaded do not
  displace each other on every import. `Provenance::supersedes` is
  unchanged. There are two limits, both in §1.4:
  - a `--files` re-read of an older file under the same date replaces the
    held copy, since its read time is the folder's;
  - a same-date correction by another route than the held copy's stays
    `Older`.
- DESIGN.md §1.4: a resource built from several of the source's records (a
  prescription from its fills, a coverage from its statements) takes the
  latest record's date, a day at `00:00:00Z`. That holds whether the
  records were read from a site or parsed from documents, and the rule
  comes before the site and document rules. Its content comes only from
  that record and from what the records share, never from the range a
  pull covered. A narrower pull then cannot displace a fuller copy, and a
  latest record that changed under its date replaces the held one.
- DESIGN.md §2.1 and `MedicationDispense`'s docs: a prescription the source
  numbers is a MedicationRequest of its own in the stream, which each of its
  fills names as `MedicationRequest/<id>` (read with `Reference::target`), as
  caremark-cli writes it. A contained MedicationRequest (`#<id>`) is only for
  a prescriber named without a prescription. v0.6.5 named the contained form
  only. A producer writes the MedicationRequest in every stream holding a fill
  that names it, whatever `--since` bounds. A store resolves the reference
  among what it has filed, from that stream or an earlier one, and keeps a
  fill whose prescription it has not filed.

## v0.6.5 — 2026-11-19

Pins cli-common v0.33.2, as v0.6.4 does.

- `pk-med-fhir`: `MedicationDispense`, a pharmacy fill, with
  `MedicationDispensePerformer` and `MedicationDispenseSubstitution` (from
  the caremark-cli session). It names the identifiers (prescription and
  claim number), status (`cancelled` for a reversed claim), the drug (NDC,
  RxNorm), the pharmacy as `performer`, `type` (first fill or refill),
  `quantity`, `daysSupply`, `whenPrepared`, `whenHandedOver`, the dosage
  text and `substitution`. A prescriber the source names without a
  prescription of its own is the `requester` of a contained
  MedicationRequest that `authorizingPrescription` points at. A synthetic
  fill joins the mock server's resources.
- DESIGN.md §1.4: a held copy with no provenance of its own (a `Binary` or
  `Practitioner` no record was filed from) is decided against a provenance
  with no known time, so other content replaces it, as medrec does.

## v0.6.4 — 2026-11-19

Pins cli-common v0.33.2, as v0.6.3 does.

- `pk-med-records`: the new `version` module covers two versions that differ
  only in `meta.lastUpdated` (DESIGN.md §1.4). A site resource is stamped
  with its read time, so every read changes its line, and a store that
  keeps replaced versions filed a copy per resource per read.
  - `version::content` is what a version is compared by: the resource
    without `meta.lastUpdated`, member order aside, every value as written.
  - `version::same_version` compares two versions by it.
  - `version::decide` gives the filing: `Unchanged` (same content, not more
    current), `Refreshed` (same content, more current: rewrite the held copy
    in place), `Replaces` or `Older`.
  - `Provenance::refreshed_by` is a refreshed version's provenance: where
    and when it was read from the incoming one, and `filed_at`, `filed_by`,
    `raw_path`, `classified_by` and `derived_from` from the held one.
- DESIGN.md §1.4: a resource the site shows with a fixed date of its own,
  such as a listed document and its `Binary`, carries that date instead of
  the read time.
- `pk-med-fhir`: `FhirClient` takes a reference as an absolute URL only when
  it starts with a scheme and `://`. Before, any `://` did it, so
  `get_raw("Condition?patient=p1&code=http://snomed.info/sct|123")`, a
  relative search with a token's system in its query, was refused as
  pointing at another server (exit 5). It now goes to the base. A
  network-path reference (`//host/…`) is still refused, as is an absolute
  URL on another origin.

## v0.6.3 — 2026-11-19

Pins cli-common v0.33.2, as v0.6.2 does.

- DESIGN.md §1.4: a resource carries `meta.lastUpdated` whenever its
  producer knows when its content was current. A FHIR source's value
  passes unchanged, and one it omits stays omitted. A resource mapped from
  a site gets the time the page was read. One parsed from a document gets
  the printed issue date, as that day at `00:00:00Z` with no time-zone
  conversion, or the month's first day if it is dated only to the month.
  A version's time is its `meta.lastUpdated`. A FHIR API resource the
  server sent undated takes its stream's `pulled_at`. A mapped one takes no
  fallback, because a `--files` pull time is when the folder was read. A
  value that is not an RFC 3339 instant counts as absent. A store keeps the
  later of two versions. A tie, or an incoming version with no known time,
  keeps the held one, and a held version with no known time is replaced, as
  the last import was before. §1.6: `source_resource.last_updated` is the
  line's `meta.lastUpdated` as written.
- `pk-med-records`: `Provenance::current_at`, that time, and
  `Provenance::supersedes`, the decision. Both read RFC 3339 through the new
  `Timestamp`, which compares instants written in different offsets or
  fractional digits correctly. As strings, `2026-02-28T19:00:00-05:00`
  sorts before `2026-03-01T00:00:00Z`, though they are the same instant.
- Already filed versions: a `fhir-api` one without `last_updated` now
  compares by its `fetched_at`. A mapped one without it has no known time,
  so the first dated version of it replaces it.
- DESIGN.md §3.1 and `SignInWindow`'s docs: Kaiser Permanente's Epic
  organizations need no page before the authorize address. v0.6.1's docs
  said a kp.org session was missing. mychart found on 2026-11-19 that the
  500 comes from the capitalized `/<region>/Authentication/OAuth/Start`
  path, with or without a kp.org session, and that the lowercase path reaches
  Kaiser's sign-in. mychart handles this itself. `before` is unchanged as a
  mechanism. No code change.

## v0.6.2 — 2026-11-17

Pins cli-common v0.33.2 for every `pk-cli-*`; a consumer moves its own
`pk-cli-*` to v0.33.2 in the same change.

- `pk-med-records`: `RECORDS_FILES` (`"records-files"`), the `cli-info/v1`
  capability of a CLI whose `patients list` and `records export` read
  downloaded files with `--files <DIR>` and no login. DESIGN.md §1.1 now
  names that source: the flag, its `<PREFIX>_FILES` fallback, how it and
  `--profile` exclude each other (both are exit 2; a flag beats the other
  source's variable), which commands take it (`patients list` and
  `records export` must, `documents` may, `api` never), what the stream
  header says (`method: portal`, no `patient.session`, no `label`, so
  `--patient` takes the id), and that both sources give a member the same
  `organization` and `patient.id`, or the files source an organization id
  of its own. Before, §1.1 said only that "a flag chooses" it,
  so a records store could not reach it.
- Pins cli-common v0.33.2, whose vault creates its `.key.lock` `0600` and
  narrows one left at the umask's mode. It was empty, in a `0700`
  directory. No API change; a consumer moves to it with the next
  medical-common release.
- DESIGN.md §1.1: `profiles migrate` asks for no new approval and its move
  makes no request. A CLI may afterwards revoke the tokens a replaced
  duplicate alone holds, never the kept copy's, and a failed revoke only
  warns. It said "no network" before. No code change.

## v0.6.1 — 2026-11-17

Pins cli-common v0.33.1 for every `pk-cli-*`; a consumer moves its own
`pk-cli-*` to v0.33.1 in the same change. No API change in medical-common.

- cli-common v0.33.1's `ConfigStore::save` writes `config.json` `0600` by
  temporary file and rename, so the profiles and grants v0.6.0 put in it are
  no longer readable by other local accounts or lost to a torn write. A
  `0644` file from before is tightened on its next save.
  `pk_cli_core::private_file` is the one owner-only writer, for a CLI that
  keeps its grants in an index of its own. A CLI that wrote its config
  through its own writer for v0.6.0 can go back to `ConfigStore::save`.
- DESIGN.md §1.1: the config and any grant index are written owner-only and
  atomically.
- DESIGN.md §1.1: a CLI keeps one record per grant, in the profile's
  settings or in an owner-only index of its own keyed by profile,
  organization and patient, never both. §3.7: a CLI's unit tests cover
  the move with `MemoryKeys` as the old store; surface tests never write the
  vault or the keychain, and with a temporary `HOME` have no keychain at
  all. No code change.
- DESIGN.md §3.1 and `SignInWindow`'s docs: the page opened before the
  authorize address is mychart's, at Kaiser Permanente's Epic
  organizations, whose Start page on `healthy.kaiserpermanente.org` needs a
  kp.org session. kaiser's own sign-in goes to `pfam.kp.org`, on another
  domain, and passes no such page. v0.6.0 named kaiser. No code change.

## v0.6.0 — 2026-11-17

Pins cli-common v0.33.0 for every `pk-cli-*`. A consumer moves its own
`pk-cli-*` to v0.33.0 in the same change. Nothing is removed: a CLI that
keeps its one-item-per-sign-in `CredentialStore` builds unchanged.

- Profiles, one per login (DESIGN.md §1.1). Every command that uses a login
  takes `--profile <NAME>` (`<PREFIX>_PROFILE`), with no default and no
  lone profile picked: exit 2 listing the configured ones, through
  cli-common's `pk_cli_config::profile::resolve`. `profiles list|add|remove`
  and `profiles migrate <NAME>`; `profiles list --json` is cli-common's
  `profile-list/v1`.
- `pk-med-records`: `LOGIN_PROFILES` (`"login-profiles"`), the `cli-info/v1`
  capability of a CLI whose login commands take `--profile`, so a records
  store refuses an older provider rather than run it unprofiled.
- `pk-med-smart`: `ProfileSessions`, a `SessionStore` over cli-common's
  `pk_cli_auth::vault::Vault`: each session the entry
  `<profile>/<organization>/<patient>`, the whole CLI under one keychain
  item, so a new binary asks macOS once rather than once per sign-in.
  `accounts()` and `delete_all()` for status and `profiles remove`.
- `pk-med-smart`: `migrate_sessions(from, to, accounts, lock)` moves
  sessions kept one keychain item each into a profile, under the refresh
  lock and with no network: each item's text copied, read back and compared
  before the item is deleted; when the profile already holds the session,
  the newer by when its tokens were obtained wins (a sign-in after a
  downgrade); idempotent. `Migration` counts what moved,
  what was already there and what was absent. Each old item read by a new
  binary costs one dialog, once; delete test items beforehand with
  `security delete-generic-password`, which raises none (§3.2).
- `pk-med-smart`, feature `browser`: `SignInWindow::open(bin, &profile,
  &sign_in, before)` and `open_at` open the sign-in in the profile's own
  kept Chrome profile (`browser-profile-<profile>`), after the pages in
  `before` (kaiser's kp.org sign-on), and close Chrome on drop so its
  cookies persist for the next sign-in (§3.1).
- `PatientContext.session` stays the account `smart/<org>/<patient>`, an
  opaque handle the CLI resolves within the command's profile.
- DESIGN.md §3.2: installs and upgrades on this Mac happen only when the
  user is present, since a new binary's first keychain read waits on a
  dialog.
- DESIGN.md §3.6: why no CLI scripts a portal's sign-in or consent page.
  The patient approves in a browser window the CLI opens, and everything
  else runs on the stored session. The section keeps each vendor's terms
  on scripted access and the protections on its sign-in page. Tests is
  now §3.7.

## v0.5.0 — 2026-11-16

Pins cli-common v0.32.0 for every `pk-cli-*`. A consumer moves its own
`pk-cli-*` to v0.32.0 in the same change.

- `pk-med-smart`: `SmartApp` is `#[non_exhaustive]` and built with
  `SmartApp::new(app, client, ports)`, which takes what the struct literal
  did; a literal no longer compiles outside the crate. Its fields stay
  public.
- `pk-med-smart`: `SmartApp::redirect_uri(uri)` sends a registered `https`
  page as the redirect instead of the listener's
  `http://127.0.0.1:<port>/callback`, for a vendor that takes only `https`
  redirects in production, as Epic does. The page sends the browser on to
  the listener with the query unchanged; `state`, `iss` and the PKCE
  verifier are checked as before, and the exchange names `uri`. The app
  names the one port the page forwards to. `begin` refuses, as exit 2, a
  `uri` that is not `https`, has no host, names a user or carries a
  fragment, and any other number of ports (cli-common's
  `AuthorizationCodeFlow::redirect_uri`). `SmartSignIn::listener_uri()` is
  where the answer must arrive (DESIGN.md §3.1).
- `pk-med-fhir` `test-util`: `MockServer::accept_redirect(uri)` accepts a
  registered `https` redirect besides the loopback (DESIGN.md §2.3).
- The personal-data scan ends an address's domain at a URL's path, port or
  query, so `https://user@host.test/path` is read as the dummy domain
  `host.test`.
- DESIGN.md §1.4: the shape of the `forbidden` skip for withheld results.
  There is one per type and distinct withheld line in a stream, with
  `resource_type` set and `id` and `search` unset, because what a server
  withholds is part of a type's data rather than one search; `search`
  stays for a search left out or left unfinished. No code change.
- DESIGN.md §1.7: a `DocumentReference` search with withheld results keeps
  what it served. `documents list` and `documents download --all` warn on
  stderr once per distinct withheld line, and their `--json` output carries
  an optional `withheld` array of the §1.4 skips, since `documents/v1` has
  no field for it. Once `documents/v1` gains a shared field, a CLI writes
  both for the rest of `/v1`.

## v0.4.9 — 2026-11-16

Pins cli-common v0.31.1, as v0.4.8 does. Additions only.

- `pk-med-fhir`: `OperationOutcome::withheld()` reads what a server says
  it left out of its answer: one line per issue coded `suppressed`,
  whatever its severity, as its `diagnostics`, else `details.text`, else
  the first coding's display, printable. Epic answers a search touching a
  sub-resource the client may not read with the rest of the results and
  such a warning, which `outcomes` carried with nothing reading it.
  `SearchResult::withheld()` and `SearchPage::withheld()` give each
  distinct line across their outcomes (DESIGN.md §2.2).
- `pk-med-fhir` `test-util`: `MockServer::withhold(type, text)` adds that
  warning, in Epic's shape, to every page of the type's searches
  (`fixtures/operation-outcome-suppressed.json`).
- DESIGN.md: a search's withheld results are a `forbidden` skip, the
  results it did send kept (§1.4); a type the source searches only by
  `_id`, as Epic's Specimen and Media, is read by reference (§1.1).

## v0.4.8 — 2026-11-16

Pins cli-common v0.31.1, as v0.4.7 does. Additions only.

- `pk-med-fhir`: `RefusedReference` carries the reference's `base` (as
  `ReferenceTarget::base`: as written, `None` when relative) and has
  `is_under(base)`, so a CLI can tell a refused reference on its own
  server from one elsewhere without splitting it again (DESIGN.md §2.1).
  Its `Debug` shows the base too, still never the id or the version.
- `pk-med-records`: `skip::INVALID_REFERENCE` (`"invalid_reference"`), the
  code for a resource left out because another names it by a reference FHIR
  does not allow: the source's own data, not a failure worth retrying as
  `upstream` would say. Skip codes are an open set, so a reader that does
  not know it treats it as any other skip (DESIGN.md §1.4).

## v0.4.7 — 2026-11-16

Pins cli-common v0.31.1, as v0.4.6 does. Additions only.

- `pk-med-fhir`: `ReferenceTarget::read(reference)` is `parse` with a
  refused id kept apart. `parse` returns `None` both for what is not a
  literal reference and for `Type/<id>` whose id `is_id` refuses, so a
  caller filtering on it dropped the second silently. `read` returns
  `Ok(None)` for the first and `Err(RefusedReference { resource_type, id,
  version })` for the second, whose `Display` and `Debug` name the type
  and the id's length, never the id (DESIGN.md §2.1).

## v0.4.6 — 2026-11-16

Pins cli-common v0.31.1, as v0.4.5 does.

- `pk-med-smart`: what to do after an error is its remedy, no longer part
  of its message, so a CLI's own ``with_remedy("run `kp auth login …`")``
  replaces it instead of following "run auth login" in the same line. Six
  errors moved: not signed in (`remedy::SIGN_IN`), a lapsed sign-in with
  no refresh token and an ID token without this sign-in's nonce
  (`SIGN_IN_AGAIN`), a stored session that does not read
  (`CLEAR_AND_SIGN_IN_AGAIN`) or is in a newer format (`UPDATE`), and a
  429 or 5xx on discovery (`TRY_AGAIN`). The new `remedy` module names
  them; `SIGN_IN_AGAIN` and `TRY_AGAIN` are spelled as pk-med-fhir's
  `outcome` constants (DESIGN.md §3).

## v0.4.5 — 2026-11-16

Pins cli-common v0.31.1, as v0.4.4 does. Additions only.

- `pk-med-fhir`: `ReferenceTarget::parse(reference)` and
  `Reference::target()` read what a literal reference names: `Type/id` or
  `Type/id/_history/vid`, relative or after an absolute `http(s)` base,
  as `base`, `resource_type`, `id` and `version`. A local, `urn:` or
  conditional reference, or a type, id or version FHIR does not allow, is
  `None`. `is_under(base)` says whether it names a resource on that base
  (DESIGN.md §2.1).

## v0.4.4 — 2026-11-16

Pins cli-common v0.31.1, as v0.4.3 does. Additions only.

- `pk-med-fhir`: `is_id(id)` is the rule `FhirClient::read` and
  `FhirClient::binary` apply to an id: 1 to `MAX_ID_LEN` of
  `A-Z a-z 0-9 - .`, not only dots. Both still refuse any other id as a
  usage error (exit 2); a CLI holding an id from a server's response checks
  it with `is_id` first and decides what a refusal means there.

## v0.4.3 — 2026-11-16

Pins cli-common v0.31.1, as v0.4.2 does.

- `pk-med-fhir`: `FhirClient::read` and `FhirClient::binary` accept an id
  of up to `MAX_ID_LEN`, 256 characters, where they refused one over FHIR
  R4's 64. An Epic app registered with "Use Unconstrained FHIR IDs",
  Epic's default, gets longer ids (80 and more). The character set
  (`A-Z a-z 0-9 - .`) and the refusal of an id of only dots are unchanged.

## v0.4.2 — 2026-11-12

Pins cli-common v0.31.1, as v0.4.1 does.

- `pk-med-smart`: `MemoryStore` is `Send + Sync`, as `CredentialStore` is,
  so a test can put it behind a `FhirClient::bearer_with` source (its map
  is behind a `Mutex`, not a `RefCell`). It keeps each item as a `Secret`,
  zeroized when replaced or dropped.

## v0.4.1 — 2026-11-12

Pins cli-common v0.31.1, as v0.4.0 does. Additions only.

- `pk-med-fhir`: `FhirClient::search_pages(type, &params, on_page)` hands
  each page to `on_page` as it is read (`SearchPage`) and returns how the
  search ended (`SearchEnd { pages, truncated, total }`). A failure on a
  later page leaves the pages before it with the caller, and an error from
  `on_page` stops the search with no further request. `search` is built on
  it and behaves as before (DESIGN.md §2.2).

## v0.4.0 — 2026-11-12

Pins cli-common v0.31.1, as v0.3.0 does.

- `pk-med-smart`: an OpenID sign-in sends a nonce (DESIGN.md §3.1). When
  the scopes ask for `openid`, `begin` adds a fresh `nonce`, 32 random
  bytes in base64url. `exchange` refuses an ID token that does not carry
  it, or carries another (exit 3, `CredentialRefused`), as OpenID Connect
  Core §3.1.3.7 requires. `SmartSignIn::nonce_checked()` says whether the
  check runs. `VendorQuirks::send_nonce` (default `true`) turns it off for
  a server that refuses the parameter. athenahealth requires a nonce with
  `openid`.
- `pk-med-fhir`: a vendor's retry guidance can be followed (DESIGN.md
  §2.2). `RetryPolicy` gains:
  - `server_errors(&[502, 503, 504])`: which 5xx are asked again (default:
    every 5xx);
  - `retry_after_required(true)`: a 429 is asked again only with a
    `Retry-After` within `max_wait` (default `false`);
  - `jitter(0.25)`: each wait lengthened by a random 0–25 %, never past
    `max_wait` (default `0`);
  - builder methods `attempts`, `delay` and `max_wait`.
- **Breaking:** `RetryPolicy` is `#[non_exhaustive]`, so a struct literal
  no longer compiles. Build it from `RetryPolicy::default()` or
  `RetryPolicy::none()` with its methods. Its fields stay public to read
  and assign.
- `test-util`: the mock's ID token carries the authorization's `nonce`.
  `MockServer::id_token_nonce_next` gives the next one another nonce, or
  none.
- `pk-med-dicom`: the doc comment on `DicomError::Limit` names
  `MAX_INFLATED_BYTES`.

## v0.3.0 — 2026-11-12

Pins cli-common v0.31.1, as v0.2.0 does.

- `pk-med-dicom` (DESIGN.md §4): our own DICOM Part 10 header reader. No
  one else's DICOM library is used, and no pixel is decoded.
  - `read_header` and `read_file`: the preamble, or a file with none; the
    file meta information; and the data set in implicit or explicit VR
    little endian, explicit VR big endian, deflated, or a compressed
    syntax's explicit little endian. Sequences of defined and undefined
    length are read, and an undefined-length `UN` is read as a sequence.
    Reading stops at Pixel Data. Limits are enforced on value size, header
    size (each element and item counting 128 bytes), inflated size
    (256 MiB) and nesting.
  - `Header`: `get(tag) -> Option<&str>` for a `Tag` or `0xGGGGEEEE`,
    `element(tag)`, and typed getters (`study_uid`, `modality`,
    `study_date` as `YYYY-MM-DD`, `patient_name` as a person writes it,
    ...). `Debug` and errors show no value.
  - `read_dicomdir` and `dicomdir_entries`: the files a DICOMDIR names,
    with their study and series. Records are walked by offset, refusing a
    dangling offset or a cycle, or in sequence order when the first root
    offset is zero. A path that leaves the disc is refused.
  - `StudySummary::from_headers`: studies and series with their counts.
    It is not a wire shape (no `Serialize`): a CLI wraps it in its own
    schema-tagged DTO.
  - `DicomError` and `Value` are `#[non_exhaustive]`. In a deflated data
    set, a stream that does not inflate is `Malformed`, and an error of the
    file under it stays `Io`.
  - `is_dicom`, `is_dicom_file`, `iso_date`, `person_name`, and
    `MEDIA_STORAGE_DIRECTORY`.
  - Feature `test-util`: `synthetic::SyntheticStudy` writes a synthetic
    study (`IMAGES/S<n>/I<m>` and a DICOMDIR) in any of four transfer
    syntaxes, and `encode`, `part10` and `Node` build any data set.

## v0.2.0 — 2026-11-12

Pins cli-common v0.31.1 for every `pk-cli-*` (`private_file` syncs before it
renames). A consumer moves its own `pk-cli-*` to v0.31.1 in the same change.

- `pk-med-smart`: SMART App Launch (DESIGN.md §3).
  - `discover`: the well-known document, falling back to `/metadata` on a
    3xx or a 4xx. Every URL it keeps must be https; plain http to this
    machine only for a server whose base is one.
  - `ScopeSet` and `ScopeVersion`: v1 and v2, explicit per-type scopes, a
    vendor's own string.
  - `begin` / `SmartSignIn`: PKCE S256, `aud` (no trailing slash),
    registered ports, the issuer checked when discovery names one
    (`issuer_checked`). `exchange` for public and `client_secret_basic`
    clients. `LaunchContext`.
  - `SmartSession` (`format` 1), one keychain item per organization and
    patient at `smart/<org>/<patient>` with `%` and `/` encoded, through
    `SessionStore` (`CredentialStore`, `MemoryStore`). Errors name no value
    and no patient id.
  - `Renewer::fresh`, `renew` and `after_refusal` under `RefreshLock`. A
    refresh asks for the scope granted. A renewer for another client is
    refused. A registered client's signer is made from the session read
    under the lock (`SignerFor`). A refused grant stamps
    `SmartSession::refused_at` and keeps the sign-in. `fresh` hands out no
    refused token: the next use tries one grant, and an accepted one
    clears it.
  - `register_dynamic_client` (registration only; the first JWT-bearer
    grant is the `Renewer`'s, so a failed grant loses no client) and
    `jwt_bearer_tokens`, with `JwtSigner` and `signed_assertion` (Epic),
    assertions living 4 minutes.
  - `VendorQuirks`.
  - Feature `rsa`: `RsaKey`, an RS384 key of at least 2048 bits (rsa 0.10),
    with `RsaKey::for_client`.
- `pk-med-fhir`: `FhirClient::bearer_with`, a token source called before
  each request and once more after a 401 to renew; `FhirClient::on_retry`,
  told of each wait before a retry in one line with no query, id or token;
  `UNREACHABLE`, the reason on an error for a request that got no complete
  HTTP answer after its retries (a failed, timed-out or broken connection;
  a reason the transport set is kept). Both closures are `Send + Sync`. One entry whose resource does not read no
  longer fails its page: `RawEntry::unreadable` says why, and
  `SearchResult::unreadable` lists them. `SearchResult` and `RawEntry` gain
  a public field each.
- `pk-med-fhir`: "sign in again" and "try again later" are the error's
  remedy (`outcome::SIGN_IN_AGAIN`, `outcome::TRY_AGAIN`), no longer part
  of its message, so a CLI replaces them with its own.
- `pk-med-records`: `create_export_file` returns an `ExportFile`. The
  stream is written beside `records.ndjson`. `ExportFile::commit(&finished)`
  syncs, renames and syncs the directory: a complete stream replaces
  `records.ndjson`, and an incomplete one goes to
  `records.incomplete.ndjson` (`INCOMPLETE_FILE`), keeping the last
  complete export. Dropped uncommitted, the file is removed, and a killed
  run's file is removed by the next export (each export locks its own).
  Breaking: it returned `(PathBuf, File)`, and renamed the empty file into
  place before anything was written.

## v0.1.0 — 2026-11-12

First release. Pins cli-common v0.31.0.

- `pk-med-records`: the `health-records/v1` profile (DESIGN.md §1).
  `Organization` (`org-list/v1`), `PatientContext` (`patient-list/v1`),
  `Provenance` (`provenance/v1`) with `Provenance::from_stream`,
  `RecordsExport` (`records-export/v1`) and `create_export_file` (created new under a temporary
  name and renamed into place, mode 0600; nothing already there is
  written through). The `records-stream/v1` stream: `Header`
  (with optional `method` and `since_unbounded`), `Trailer` with counts, skips and
  `complete`, `Skip` and its codes, `RecordsWriter` (dedupe by type and
  id, 64 MiB line limit, `complete: false` on a failure after the header)
  and `RecordsReader` (bounded lines; refuses a missing header or
  trailer, a count mismatch, a line after the trailer, an incomplete
  stream). `RecordsWriter::finish` returns `Finished` (the trailer and
  the stream's whole size); the reader classifies a line exactly as the
  writer does, so every line written reads back. Feature `test-util`:
  `write_synthetic_stream`.
- `pk-med-fhir`: our own FHIR R4 / US Core types for 19 resource types
  and their datatypes, each keeping unknown elements in `extra`;
  ExplanationOfBenefit and Coverage in the CARIN Blue Button shape;
  `RawResource` and `RawBundle` keep the server's own text, so decimals
  keep their spelling; `Resource::find_contained`; `Binary::bytes`;
  `Money::to_money` (plain decimal text); `normalize_base`;
  `describe_json_error`, so no error quotes a record's value. Repeating
  primitives are `Vec<Option<T>>`, keeping FHIR's null placeholders. `FhirClient` over
  `pk_cli_core::transport::Transport`: read, search with `next` paging
  under a page cap, Binary as FHIR JSON, `get_raw`; requests only to the
  base's origin; retries a failed connection, a 429 or a 5xx under `RetryPolicy` with
  `Retry-After`; `.` and `..` segments refused however they are spelled; HTTP
  status and OperationOutcome mapped to exit codes 3, 4 and 5. Feature
  `test-util`: a loopback mock SMART + FHIR server and a synthetic
  patient with every resource type.
