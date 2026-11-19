# medical-common: the `health-records/v1` profile

Status: v0.1.0 ships `pk-med-records` and `pk-med-fhir`; v0.2.0 adds
`pk-med-smart` (SMART App Launch, §3); v0.3.0 adds `pk-med-dicom` (DICOM
headers, §4); v0.4.0 adds the OpenID nonce (§3.1) and a vendor's retry
guidance (§2.2); v0.4.1 adds `search_pages` (§2.2); v0.4.2 makes
`MemoryStore` `Sync` (§3.2); v0.4.3 reads ids longer than FHIR's 64
characters (§2.2); v0.4.4 exports the id rule as `is_id` (§2.2); v0.4.5
adds `ReferenceTarget`, what a literal reference names (§2.1); v0.4.6
keeps pk-med-smart's remedies apart from its messages (§3); v0.4.7 adds
`ReferenceTarget::read`, which keeps a refused id apart (§2.1); v0.4.8
gives a refused reference its base and `is_under` (§2.1) and adds the
`invalid_reference` skip code (§1.4); v0.4.9 reads what a search's answer
says it left out (`withheld`, §2.2); v0.5.0 moves to cli-common v0.32.0 and
sends a registered `https` page as the redirect (`SmartApp::redirect_uri`,
§3.1); v0.6.0 moves to cli-common v0.33.0 and adds profiles, one per login
(§1.1), with every session in the CLI's one-item vault and the move into
it (§3.2), and a sign-in window per profile (§3.1); v0.6.1 moves to
cli-common v0.33.1, whose `ConfigStore::save` writes the config, and with it
each profile's grants, owner-only and atomically (§1.1); v0.6.2 moves to
cli-common v0.33.2 and names the downloaded-files source, `--files <DIR>`
with the `records-files` capability (§1.1); v0.6.3 dates every resource
with `meta.lastUpdated` and compares versions by it
(`Provenance::supersedes`, §1.4); v0.6.4 sets apart a re-read that changed
only its stamp (`version::decide`, §1.4); v0.6.5 types a pharmacy fill
(`MedicationDispense`, §2.1). §3.6 says why no portal sign-in is
scripted.

This document is the specification of the profile, in the style of
cli-common's domain profiles (`documents/v1`, `utility/v1`, cli-common
`DESIGN.md` §1.8). It builds on **piekstra-cli/1**: global flags, `--json`,
one JSON document per invocation, exit codes 0–6, secrets in the keychain
only. Nothing here restates those rules; where this profile is silent,
piekstra-cli/1 holds.

## 1. The `health-records/v1` profile (crate `pk-med-records`)

For any CLI that reads a person's health records from a provider, or their
coverage and claims from an insurer: `mychart` (Epic), `athena`
(athenahealth), `uhc` (UnitedHealthcare), and the CLIs after them. The
records store (`medrec`) is the consumer. Every command is a read, so the
confirmation rules of piekstra-cli/1 §1.3 do not apply. A CLI declares the
profile in `info` (`profiles: ["health-records/v1", "documents/v1"]`).

### 1.1 Commands

```text
<bin> profiles list                                         # profile-list/v1
<bin> profiles add <NAME>                                   # one profile per login (below)
<bin> profiles remove <NAME>                                # its sign-ins, browser profile and config; asks first
<bin> profiles migrate <NAME>                               # sign-ins kept before profiles, into <NAME>
<bin> orgs list                                             # org-list/v1
<bin> orgs search <TEXT>                                    # org-list/v1
<bin> auth login --profile <NAME> --org <ORG> [--label <WHO>]   # browser sign-in, PKCE, loopback redirect
<bin> auth status --profile <NAME>                          # auth-status/v1
<bin> auth logout --profile <NAME>
<bin> patients list --profile <NAME>                        # patient-list/v1
<bin> records export --profile <NAME> --patient <ID> [--since <YYYY-MM-DD>] --ndjson    # records-stream/v1 on stdout
<bin> records export --profile <NAME> --patient <ID> [--since <YYYY-MM-DD>] -o <DIR>    # the same stream to <DIR>/records.ndjson (records.incomplete.ndjson if it ends incomplete); records-export/v1 on stdout, its path the file written
<bin> patients list --files <DIR>                           # a source of downloaded files (below), no login
<bin> records export --files <DIR> --patient <ID> …         # the same, from those files
<bin> documents list --profile <NAME>                       # documents/v1 (§1.7)
<bin> documents download --profile <NAME> …                 # documents/v1 (§1.7)
<bin> api --profile <NAME> GET <PATH>                       # passthrough, GET only
```

- `orgs search` matches `name`, `aliases` and `location`, case-insensitive.
- `--patient` takes a `PatientContext.id`, or its `label`. With one
  context signed in, it may be omitted.
- `--since` bounds each search by the date parameter the source documents
  for the type, and leaves a type the source documents none for unbounded
  rather than guess one (`_lastUpdated` can turn a whole search into a 400).
  The stream header names those types in `since_unbounded`, so a reader
  knows their resources may be older than `since`. It never drops a
  resource the source returns. (At Epic, for example, `date` is documented
  on Observation, DiagnosticReport, DocumentReference, Encounter,
  Immunization and Procedure.)
- A type the source searches only by `_id` is read by reference from the
  resources that name it, not searched by patient. (At Epic, Specimen and
  Media take only `_id`; a `patient` or `subject` search answers 400.)
- An insurer CLI's export carries `Coverage` and `ExplanationOfBenefit`
  (CARIN Blue Button). A CLI whose source is not a FHIR server (a member
  site, downloaded files) maps what it reads into those FHIR shapes and sets
  the header's `method` to `portal`, so the store sees one format.
- `--ndjson` and `-o` are exclusive; one of them is required. `--json` with
  `--ndjson` is a usage error (exit 2): the stream is the output.
- `auth status` reports `method: "browser-session"` for a SMART sign-in
  (cli-common `pk_cli_auth::oauth`).

**Profiles.** A CLI keeps one profile per login: an athenahealth account, a
MyChart account at one organization, a kp.org account, a member-site
account. Profile names are the household's own (`adult1`, `adult2`) and
live in the CLI's local config only (cli-common `pk_cli_config::profile`,
its DESIGN.md §2.5).

- Every command that uses a login, a stored sign-in or records pulled
  through one requires `--profile <NAME>`, with `<PREFIX>_PROFILE` as the
  environment's fallback: `auth login|status|logout`, `patients list`,
  `records export`, `documents list|download`, `api`. `orgs list|search`,
  `profiles` and `info` take none. There is no default profile, and a lone
  profile is never picked: a missing or unknown one is a usage error
  (exit 2) listing the configured ones (`resolve`), or, with none, naming
  `<bin> profiles add <NAME>`. A command whose only source is files takes
  none.
- Downloaded files are the other source, for a CLI that can read records
  from files the owner downloaded (an insurer's statements, a portal's
  record download): `--files <DIR>`, with `<PREFIX>_FILES` as its
  environment fallback. A CLI may keep an older name as an alias. On a
  command that reads records, the CLI accepts `--files` in place of
  `--profile`: `patients list` and `records export` must, and
  `documents list|download` may when the files hold documents (§1.7).
  `api` is a passthrough to the login's server and takes only `--profile`.
  A command that reads records takes exactly one source:
  - both flags are a usage error (exit 2);
  - a flag wins over the other source's variable;
  - both variables and no flag are exit 2;
  - neither is exit 2, naming `--files` as well as the configured profiles.

  With `--files`, nothing reads the keychain or the vault, and no request is
  made. The stream header's `method` is `portal` (§1.4) and its
  `patient.session` is absent. A label belongs to a profile's grant, so
  `patients list --files` lists no `label`, and `--patient` takes the
  `PatientContext.id` only. With one member in the files, it may be
  omitted, as with one context signed in.

  A CLI with both sources gives a member the same identity on each: the
  same `organization` and the same `patient.id`, derived from the member
  identifier both carry. A store then files the two streams as one patient
  at one source and merges the resources they share (§1.4). A CLI whose
  sources cannot agree gives the files source an `organization.id` of its
  own and says so in its docs, so a store keeps the two apart rather than
  splitting one member's records under two ids.
- `info --json` lists `"records-files"` (`pk_med_records::RECORDS_FILES`)
  in `cli-info/v1`'s `capabilities` once `patients list` and
  `records export` take `--files`. A records store passes
  `--files <DIR>`, and no `--profile`, only to a provider that lists it.
- `profiles list [--json]` is cli-common's `profile-list/v1`:
  `{"schema": "profile-list/v1", "profiles": [{"name": "adult1"}, …]}`,
  sorted by name. `profiles add <NAME>` adds one. `profiles remove <NAME>`
  removes its sign-ins (`ProfileSessions::delete_all`, §3.2), its browser
  profile (`pk_cli_browser::forget_named_profile`) and its config, and
  asks first (piekstra-cli/1 §1.3: `--force`, exit 6 when it cannot ask).
- A profile holds zero or more grants, each an organization and a patient
  label (`--label`) with the patient's id. A CLI keeps one record per
  grant, in one place: the profile's settings (`Profiles::settings`), or an
  owner-only index of its own beside the config, keyed by profile,
  organization and patient (athena's `sessions.json`). In the second case
  the profile's settings stay `{}`, so the two never drift. Either file names
  the household's patients, so it is written `0600` and replaced
  atomically: `ConfigStore::save` does both from cli-common v0.33.1, and
  `pk_cli_core::private_file` is the writer for a CLI's own index. Its sessions
  are in the CLI's vault, one entry per grant (§3.2); relabelling a grant
  moves no entry.
- `profiles migrate <NAME>` moves the sign-ins a CLI kept before profiles
  (one keychain item each) into the profile, adding it when it is new. It
  asks for no new approval, and the move itself makes no request
  (`migrate_sessions`, §3.2). When the move replaces a duplicate, a CLI may
  then revoke the tokens that the dropped copy alone holds. It never
  touches the kept copy, and a failed revoke is a warning, not an error.
  Revoking a token the kept copy shares can end the whole grant (athena's
  `revoke_replaced`).
- `info --json` lists `"login-profiles"` (`pk_med_records::LOGIN_PROFILES`)
  in `cli-info/v1`'s `capabilities` once the CLI's login commands take
  `--profile`. A records store passes `--profile` only to a provider that
  lists it, and refuses an older one rather than run it unprofiled.
- Each profile signs in in its own Chrome profile, so one login's cookies
  never sign in another (`SignInWindow`, §3.1).

### 1.2 `Organization` (`org-list/v1`)

`orgs list` and `orgs search` emit `Paged<Organization>`
(`{"schema": "org-list/v1", "items": [...]}`), built by
`pk_med_records::org_list`. The same object is the stream header's
`organization`.

| Field | | |
|---|---|---|
| `id` | required | the CLI's own stable id: Epic's organization id, a slug |
| `name` | required | the organization's name as its directory gives it |
| `fhir_base` | required | the FHIR base, normalized by `pk_med_fhir::normalize_base` (scheme and host lower case, no trailing slash); a source that is not a FHIR server puts its site's base here |
| `vendor` | optional | `epic`, `athenahealth`, `cerner`, `uhc`, ... |
| `location` | optional | city and state, or a region, to tell same-named organizations apart |
| `aliases` | optional | other names: a brand, a former name |

### 1.3 `PatientContext` (`patient-list/v1`)

A patient at a provider, or a member at an insurer, that the CLI holds a
sign-in for. `patients list` emits `Paged<PatientContext>` (`patient-list/v1`,
`pk_med_records::patient_list`). The same object is the stream header's
`patient`.

| Field | | |
|---|---|---|
| `id` | required | the FHIR `Patient.id` at the source |
| `organization` | optional | the `Organization.id` it is at |
| `label` | optional | the household's own name for it (`--label`), never read from the record |
| `relationship` | optional | `self`, or `proxy` for a parent's access to a child's record |
| `kind` | optional | `patient` at a provider, `member` at an insurer |
| `member_role` | optional | `subscriber` or `dependent` |
| `fhir_user` | optional | the signed-in user's `fhirUser` (`Patient/…`, `RelatedPerson/…`) |
| `name` | optional | the name the source shows, for the person's own terminal; never in a fixture, a log or an error message |
| `session` | optional | the sign-in's account, `smart/<org>/<patient>` (`pk_med_smart::account`, §3.2), never the token. An opaque handle: the CLI finds the session by it within the profile the command ran as, in the vault entry `<profile>/<org>/<patient>`, or for a CLI before profiles in the keychain item of that name |

### 1.4 The records stream (`records-stream/v1`)

`records export --ndjson` writes NDJSON to stdout: a header line, one FHIR
resource per line, and a trailer line. The writer is
`pk_med_records::RecordsWriter`; the reader is `RecordsReader`.

```text
{"schema":"records-stream/v1","source":{"cli":"mychart","version":"0.1.0"},"organization":{...},"patient":{...},"pulled_at":"2020-01-03T00:00:00Z","fhir_version":"4.0.1"}
{"resourceType":"Patient","id":"pat-0001",...}
{"resourceType":"DocumentReference","id":"doc-0001",...}
{"resourceType":"Binary","id":"bin-0001","contentType":"application/pdf","data":"JVBERi0..."}
{"schema":"records-stream-end/v1","complete":true,"counts":{"Binary":1,"DocumentReference":1,"Patient":1},"skipped":[],"finished_at":"2020-01-03T00:01:00Z"}
```

**Header** (`Header`):

| Field | | |
|---|---|---|
| `schema` | required | `records-stream/v1` |
| `source` | required | `{cli, version}`: the program that wrote the stream |
| `organization` | required | an `Organization` (§1.2) |
| `patient` | required | a `PatientContext` (§1.3) |
| `since` | optional | `--since`, `YYYY-MM-DD` |
| `since_unbounded` | optional | with `since`: an array of FHIR `resourceType` names (`["Patient", "Medication"]`), the types the pull left unbounded because the source documents no date parameter for them; omitted when empty and when `since` is absent |
| `pulled_at` | required | when the pull started, RFC 3339 UTC |
| `fhir_version` | defaulted | `4.0.1`; a reader takes an absent one as `4.0.1` |
| `method` | optional | how the resources were obtained: absent means `fhir-api`; `portal` for resources mapped from a member or patient site or from files downloaded there |

**Resource lines.** A line is a resource if and only if it has a top-level
`resourceType`. Each is the server's own JSON text, minified (whitespace
outside strings removed) and otherwise unchanged, so a decimal keeps its
spelling (`1.50` stays `1.50`; FHIR gives the spelling meaning). A producer
writes `pk_med_fhir::RawResource` and never re-serializes a parsed value.

- Every type the source serves goes in, `Binary` included. A `Binary` is
  fetched as FHIR JSON (content in base64 `data`) and written right after
  the first `DocumentReference` or `DiagnosticReport` that names it.
- A resource appears once per stream, by `(resourceType, id)`; the writer
  drops repeats. A resource with no `id` is always written.
- A producer that maps a non-FHIR source keeps each resource's id stable
  across runs and import routes (derived from the source's own claim or
  document number), so the store's same-resource merge works. It marks each
  mapped resource with `meta.source` (`uhc:portal`, `uhc:eob-pdf`) and a
  `meta.tag` saying it was mapped.
- A resource carries `meta.lastUpdated`, a FHIR `instant`, whenever the
  producer knows when its content was current. It dates the content and
  says nothing about how complete it is. The value by source:
  - A FHIR source's resource keeps the server's value as served, since the
    line is the server's text. One the server sent without it stays
    without.
  - A resource mapped from a member or patient site has `meta.lastUpdated`
    set to the time the page was read, or the header's `pulled_at`. A
    resource the site shows with a fixed date of its own carries that date
    instead, as a document does: a document listed with the date it was
    issued or created, and its `Binary`. The read time changes the line on
    every read; the store's rule for the same version (below) keeps that
    from filing a copy per read.
  - A resource parsed from a document (a statement, a downloaded record)
    has it set to the date printed on the document as its issue date. That
    calendar date is taken as printed, with no time-zone conversion, and
    written as that day at `00:00:00Z`; a document dated only to the month
    takes the month's first day. A document ranks as the start of its day,
    so a site read on the day it is issued ranks after it. A producer's
    docs name the printed date it takes for each kind of document.
  - A resource a producer builds from several of the source's records (a
    prescription from its fills, a coverage from its statements) takes the
    date of the latest of them, a day written as `00:00:00Z`. Its content
    comes from that record and from what the records share, never from
    the range a pull covered (an earliest fill, a count of fills). Two
    copies with the same date are then the same, so the tie that keeps the
    held one loses nothing, and a narrower pull cannot displace a fuller
    copy.
  - Otherwise it is left out.

  A version's time is its `meta.lastUpdated`, read as an RFC 3339 instant
  (`pk_med_records::Timestamp`, so two offsets or precisions compare
  correctly). A resource from a FHIR API (`method` `fhir-api`) that the
  server sent without one takes its stream's `pulled_at`, the time it was
  last seen current. A mapped resource takes no such fallback: for a
  `--files` import the pull time is when the folder was read, not when a
  statement was current. A value that does not read as an instant counts
  as absent. A store holding one version of a resource, given another,
  decides by `Provenance::supersedes`:

  | Held | Incoming | Kept |
  |---|---|---|
  | time known | time known and later | incoming |
  | time known | time known, the same or earlier | held, so a re-import changes nothing |
  | time known | time unknown | held |
  | time unknown | either | incoming: the last import wins, as before versions were dated |

  Two versions whose content matches in everything but `meta.lastUpdated`
  are the same version, as a site read again unchanged is. The content
  (`pk_med_records::version::content`) is the resource without
  `meta.lastUpdated`, a `meta` left empty by that counted as absent. The
  order of its members and of `meta`'s does not count; every value is
  compared as written, so a decimal's spelling does. `version::decide` gives
  the store's filing. A refresh takes where and when the version was read
  from the incoming provenance (`source`, `method`, `source_resource`,
  `fetched_at`) and keeps what the store decided (`filed_at`, `filed_by`,
  `raw_path`, `classified_by`, `derived_from`), so a hand classification
  survives every re-read. A held copy the store keeps with no provenance of
  its own (a `Binary` or `Practitioner` no record was filed from) is decided
  against a provenance with no known time, so other content replaces it:

  | Content | Incoming supersedes held | Filing | The store |
  |---|---|---|---|
  | same | no | `Unchanged` | files nothing |
  | same | yes | `Refreshed` | rewrites the held copy in place and refreshes its provenance (`Provenance::refreshed_by`), adding no version |
  | differs | yes | `Replaces` | makes the incoming version current; it may keep the held one as an earlier version |
  | differs | no | `Older` | keeps the held version current; it may keep the incoming one as an earlier version |

**Trailer** (`Trailer`):

| Field | | |
|---|---|---|
| `schema` | required | `records-stream-end/v1` |
| `complete` | required | `true` when the pull finished; `false` when it stopped on an error after the header |
| `counts` | required | resource lines written, by `resourceType` |
| `skipped` | required | what was left out and why (`Skip`), possibly empty |
| `finished_at` | optional | RFC 3339 UTC; a writer always sets it |
| `error` | optional | with `complete: false`: `{code, message, exit_code?}`, `code` being piekstra-cli/1's error code (`auth`, `not_found`, `upstream`, ...) |

**`Skip`**: `{resource_type?, id?, search?, reason, code}`. `search` names a
search left out (`Observation?category=laboratory`) with no patient id or
token in it; `reason` is for a person and carries no record content. `code`
is from an open set; a reader treats an unknown code as a skip like any
other:

| `code` | Meaning |
|---|---|
| `no_file` | the resource names a file the source did not serve |
| `verify_failed` | a file failed its check (`documents/v1`'s `verify_download`) |
| `upstream` | the source failed: a 5xx, a 429 not waited out |
| `forbidden` | the source refused access: a 403, a scope not granted, or a search's answer said it left results out (an issue `suppressed`, `SearchResult::withheld`, §2.2) |
| `not_found` | a 404 or 410 for something a resource named |
| `too_large` | over the line limit |
| `unsupported` | the source does not offer the search or the type |
| `page_cap` | pages were left at the page cap |
| `invalid_reference` | a resource names another by a reference FHIR does not allow (`ReferenceTarget::read`'s `Err`, §2.1): the source's own data, which asking again does not change |

A search answered with results and an issue saying it left some out keeps
those results, and the stream carries one `forbidden` skip per type and
distinct withheld line, however many searches carried it. What a server
withholds is part of a type's data (at Epic, a sub-resource such as a
Condition category), not one search, so the skip sets `resource_type`,
leaves `id` and `search` unset, and gives the line as `reason`, which may
also say which searches carried it (`Skip::resource(type, "",
skip::FORBIDDEN, line)`). `search` stays for a search left out or left
unfinished. A reader takes a `forbidden` skip with a `resource_type` and
neither `id` nor `search` as resources of that type the source did not
serve; those it served are in the stream.

**Failure.** Nothing goes to stdout before the header, so a failure before
the first request (no sign-in, a bad argument) is an ordinary error: the
error DTO, exit 2, 3, 4 or 5. Once the header is out, the stream is the
output: a failure ends it with a trailer of `complete: false` and its
`error`, and the CLI exits with that error's code (3, 4 or 5), writing no
second JSON document. A failure of one search or one file is a `Skip`, not a
stop.

**Limits.** No line is over `MAX_LINE_BYTES`, 64 MiB (a Binary of about 48
MiB), not counting its line ending. The writer skips a larger resource as
`too_large` (a writer's limit can be lowered, never raised past it); the
reader refuses a larger line as `LineTooLong` without holding it in memory.

**One reading on both sides.** The writer takes a resource that
`RawResource::parse` reads: a JSON object with a string `resourceType`, its
other members checked as JSON and not otherwise read. The reader classifies
each line the same way and never builds a `serde_json::Value` of a
resource, so every line a writer wrote reads back. A value a `Value`
refuses (`1e400`, a lone `\ud83d`) stays the resource's own text, for the
store to keep or refuse per resource.

**Reading.** `RecordsReader` yields the header, each resource as a
`RawResource`, then the trailer. It refuses, as `StreamError`, each mapped
to exit 5:

| Error | When |
|---|---|
| `MissingHeader` | the first non-blank line is not a `records-stream/v1` header |
| `BadHeader` | the header names the schema but does not read; after it, the reader yields nothing |
| `BadJson { line }` | a line is not JSON, or the trailer does not read |
| `LineTooLong { line }` | a line is over the limit |
| `NotAResource { line }` | a line is not an object, or has neither a string `resourceType` nor the trailer's schema |
| `AfterTrailer { line }` | a line follows the trailer |
| `MissingTrailer` | the input ended with no trailer: the stream was cut short |
| `CountMismatch` | the trailer's count for a type is not the number of lines read |
| `Incomplete { trailer }` | the trailer says `complete: false`; every resource before it was yielded |

Blank lines and CRLF endings are accepted. A store files what it read from
an incomplete stream as such, or not at all; it never takes a stream without
its trailer as whole.

No error, here or anywhere in these crates, quotes a value from a record.
serde's messages do (`invalid type: string "…"`), so they are rewritten by
`pk_med_fhir::describe_json_error`: what kind of fault, a missing element's
name, and the line and column.

### 1.5 `records export -o <DIR>` (`records-export/v1`)

The same stream, to `<DIR>/records.ndjson`, by
`pk_med_records::create_export_file`, which returns an `ExportFile`. The
directory is created with mode 0700 when missing, and the stream is written
to a new file with mode 0600 under a temporary name in it (`O_EXCL`, which
follows no link). `ExportFile::commit(&finished)`, called with what
`RecordsWriter::finish` returned, syncs the file, renames it into place and
syncs the directory:

- A complete stream goes to `records.ndjson`, and removes any
  `records.incomplete.ndjson`, which is older.
- An incomplete one (`complete: false`: a provider's error stopped the
  pull) goes to `records.incomplete.ndjson`, and the last complete export
  at `records.ndjson` is kept.

Until the commit nothing at either name changes, so an export that is
killed, runs out of space or fails leaves the last complete one. An
`ExportFile` dropped without a commit removes its file. A run killed before
that leaves it, hidden as `.records.ndjson.<pid>.<nanos>.<n>.tmp`: each
export holds a lock on its own file while it runs, and the next export to
the directory removes every such file of the same owner that no live
process holds. A filesystem without locks (some network mounts) gets no
sweep, since a leftover there cannot be told from a live export. An ordinary file or a hard link already at either name is
replaced by the rename and never written through. A symbolic link at
`records.ndjson` is refused, never followed: exit 2 (`CliError::Usage`), and
nothing is written. stdout gets one `records-export/v1` document (`path` being where `commit`
put it): `{schema, path,
bytes, complete, counts, skipped?}`, `bytes` being the whole file, trailer
included (`RecordsExport::new(path, &finished)`, from what
`RecordsWriter::finish` returned). Development and tests use it with
synthetic data only.

### 1.6 Provenance (`provenance/v1`)

What the store keeps on every record it files. `Provenance::from_stream`
fills it from a stream's header and one resource; the store adds the rest.

| Field | | |
|---|---|---|
| `source` | required | `{kind, cli?, cli_version?, organization_id?, fhir_base?}`; `kind` is `fhir`, `portal`, `scan`, `drive`, `email`, `media`, `manual` or `huishouden` |
| `method` | required | `fhir-api`, `portal`, `scan`, `file`, `media`, `manual` or `huishouden` |
| `source_resource` | optional | `{type, id, version_id?, last_updated?}` of the resource at the source: `version_id` and `last_updated` are the line's `meta.versionId` and `meta.lastUpdated` as written, each absent when the line has none |
| `fetched_at` | optional | the stream's `pulled_at` |
| `filed_at` | required | when the store filed it, RFC 3339 |
| `raw_path` | optional | the raw file in the records folder: `<Person>/Records/<source>/<Type>/<id>.json`, or the document as received |
| `classified_by` | optional | `medrec` (its rules), `organize-scans`, or `hand` (the command line or review) |
| `derived_from` | optional | `{document_id, rule_id}` for a field derived from a document |
| `filed_by` | required | the program and version that filed it (`medrec 0.3.0`) |

From a stream, `method` is the header's (`fhir-api` when absent) and
`source.kind` is `fhir` for `fhir-api`, otherwise the method itself.
When a version's content was current is `last_updated`, or for a
`fhir-api` version without one, `fetched_at` (`Provenance::current_at`,
§1.4).

### 1.7 How it composes with `documents/v1`

A provider CLI also implements cli-common's `documents/v1` over its
`DocumentReference` and `Binary` resources, and an insurer CLI over its EOB
files, so `organize-scans` and anything else that speaks `documents/v1`
works unchanged:

- `Document.id` is the `DocumentReference.id`; a reference with more than
  one attachment lists each as `<id>#<n>`, `n` counting from 1 in
  `content` order.
- `Document.name` is the attachment's `title`, else the reference's type
  text; `date` is `DocumentReference.date` as `YYYY-MM-DD`; `category` its
  first category's text; `file` the name the download writes.
- `documents download` fetches the attachment's `Binary` (through
  `FhirClient::binary`), decodes it (`Binary::bytes`) and passes the bytes
  through `verify_download` before writing, as `documents/v1` requires.
  A file that fails is skipped with `verify_failed`, in a batch and in a
  stream alike.
- A `DocumentReference` search whose answer says it left results out
  (`SearchResult::withheld`, §2.2) keeps what it served. `documents list`
  and `documents download --all` warn on stderr, once per distinct withheld
  line, and their `--json` output (`document-list/v1`,
  `document-download-batch/v1`) carries `withheld`: the §1.4 skips for it
  (`{resource_type, reason, code: "forbidden"}`, one per type and distinct
  line), omitted when empty. `documents/v1` has no field for this
  (`skipped` needs a document id), so `withheld` is this profile's
  extension, and a `documents/v1` reader that does not know it ignores it.
  When `documents/v1` gains a shared field for what a source left out, a
  CLI writes it beside `withheld` for the rest of `/v1`, and a reader
  prefers the shared field when present; `withheld` goes only with a schema
  bump (§1.9).
- The records stream carries the same `Binary` resources verbatim, so the
  store does not call `documents download` for a pulled patient; the two
  routes agree on ids, and a store matches a filed document to its
  reference by `Document.id`.

### 1.8 Money

Amounts are `pk_cli_core::Money`: a string decimal and a currency.
`pk_med_fhir::Money::to_money` converts a typed FHIR `Money` to plain
decimal text (never an exponent). A whole or one-place amount gains its
second place (`12.5` is `12.50`), and a missing currency is `USD`, the only
one CARIN Blue Button allows. The typed layer reads a decimal as a 64-bit
float, so up to 15 significant digits are exact and trailing zeros past the
second place are not kept (`12.100` is `12.10`). The server's own spelling
is in the `RawResource`, which a store keeps.

### 1.9 Versioning

Each schema above is `/v1` and follows cli-common's rule: a field may be
added within `/v1` only if it is optional, defaulted when read and omitted
when empty. Anything else (a removal, a rename, a new meaning) is `/v2`. A
reader ignores members it does not know, in the header, the trailer and
every DTO. The Rust structs have public fields; build them with their
constructors (`Header::new`, `Skip::resource`, `Skip::search`) so a field
added within `/v1` does not break a caller.

The header's `method` and `since_unbounded` are such fields: optional,
omitted when absent or empty, and ignored by a reader that predates them,
so `records-stream/v1` holds with or without them.

## 2. FHIR types and client (crate `pk-med-fhir`)

### 2.1 Types

Our own serde types for the FHIR R4 / US Core subset the household's
records use: Patient, Encounter, Observation, DiagnosticReport, Condition,
MedicationRequest, MedicationDispense (a pharmacy fill; its prescriber is
the requester of the MedicationRequest its `authorizingPrescription` names:
a prescription the source numbers is its own resource in the stream,
`MedicationRequest/<id>`, shared by its fills; a prescriber named without
a prescription is the requester of a contained one, `#<id>`. A producer
writes the MedicationRequest in every stream holding a fill that names it,
whatever `--since` bounds, as a resource read by reference is (§1). A
store resolves the reference by `(resourceType, id)` among what it has
filed, from this stream or an earlier one, and keeps a fill whose
prescription it has not filed), MedicationStatement, AllergyIntolerance, Immunization,
Procedure, DocumentReference, Binary, Coverage, ExplanationOfBenefit (CARIN
Blue Button: lines, adjudication amounts, totals, payment, benefit
balances), ImagingStudy, Practitioner, Organization, Provenance, Bundle and
OperationOutcome, and the datatypes they share. No FHIR library of anyone
else's.

- Fields are the elements a consumer reads, `snake_case` over FHIR's
  `camelCase`; every other element (extensions, `_field` primitive
  extensions, choice types not named, later profiles' elements) is kept in
  each type's `extra` map, so a resource read and written back holds the
  same elements (a test checks this for every synthetic resource).
- A repeating primitive (`HumanName.given`, `Address.line`,
  `Meta.profile`, ...) is `Vec<Option<T>>`: FHIR writes `null` in it where
  only the `_given` extension has a value, and the nulls are kept so the two
  arrays stay aligned.
- A decimal is a `serde_json::Number`: the value survives a typed round
  trip, its spelling may not. The spelling is kept by `RawResource`, which
  never parses a number; streams and stores are written from it.
- `Resource::contained()` and `find_contained::<R>("#id")` read contained
  resources, as CARIN EOBs carry their payer and coverage.
- `RawBundle` reads a search page keeping each entry's resource raw.
- `ReferenceTarget::parse` (and `Reference::target`) reads what a literal
  reference names: `Type/id` or `Type/id/_history/vid`, relative or after
  an absolute `http(s)` base. It returns the base as written (`None` when
  relative), the type, the id and the version. A local `#id`, a `urn:`
  reference, a conditional `Type?…` one, and a type, id or version FHIR
  does not allow (the id and version by `is_id`, §2.2) are `None`; so is a
  base with an empty, `.` or `..` segment. `is_under(base)` says whether
  it names a resource on that base, compared in `normalize_base`'s
  spelling.
- `ReferenceTarget::read` keeps a refused id apart, for a CLI that reports
  a reference it cannot follow rather than drop it: `Ok(Some(target))` as
  `parse`, `Err(RefusedReference { base, resource_type, id, version })`
  for one shaped as `[base/]Type/id` whose id or version `is_id` refuses,
  and `Ok(None)` for everything else `parse` refuses, a disallowed type
  included. A refused reference has `is_under(base)` too, so a CLI reports
  only those on its own server. Its `Display` names the type and the id's
  length; its `Debug` also shows the base (a server URL) and whether a
  version is present. Neither shows the id or the version (a patient's id
  can be an identifier); the fields hold both. A stream reports one it
  leaves out with `skip::INVALID_REFERENCE` (§1.4). Which bases a CLI follows and which types it reads stay the
  CLI's. A DocumentReference's `attachment.url` is a URL, not a reference
  (it may carry a query): `FhirClient::binary` takes it as it is.

### 2.2 `FhirClient`

A read client generic over `pk_cli_core::transport::Transport`, so the same
code runs over `pk_cli_http::BlockingTransport` (driven with
`pk_cli_core::block_on`) and any other transport.

- `read(type, id)`, `read_as::<R>(id)`, `search(type, &params)`,
  `search_pages(type, &params, on_page)`, `binary(reference)`,
  `get_raw(path)`.
- `search` adds no parameter of its own, follows `Bundle.link[next]` as the
  server wrote it, up to a page cap (`page_cap`, default 100), and reports
  `pages`, `truncated` (pages left at the cap), `total`, and its entries
  sorted by search mode: `resources` (`match`), `included` (`include`),
  `outcomes` (`outcome`). An entry whose resource does not read (not an
  object, no string `resourceType`) costs only itself: it is left out, and
  why is in `unreadable`, naming no value of it. `RawBundle::parse` keeps
  such an entry with `RawEntry::unreadable` set.
- `search_pages` is the same search, handing each `SearchPage { number,
  resources, included, outcomes, unreadable, total }` to `on_page` as it is
  read, and returning `SearchEnd { pages, truncated, total }`. A page that
  fails ends the search with its error after the pages before it were
  handed over, so an export keeps them and records one `Skip` for the
  rest. An error from `on_page` (a full disc) ends it with no further
  request. `search` is `search_pages` collecting every page, so an error on
  any page there is the whole search's.
- A server can answer a search with some of its results and an outcome
  saying it left the rest out: an issue coded `suppressed`, which Epic
  sends as a warning naming the sub-resource the client may not read, on
  every search that touches it. That is a 2xx and no error.
  `OperationOutcome::withheld()` gives one line per such issue, whatever
  its severity: its `diagnostics`, else `details.text`, else the first
  coding's display, printable and cut to 200 characters.
  `SearchResult::withheld()` and `SearchPage::withheld()` give each
  distinct line across their outcomes, so a warning repeated on every page
  is one line.
- The bearer is fixed (`bearer(Secret)`) or comes from a source
  (`bearer_with`), for a run longer than the token lasts. A source is
  called before every request, a retry included, with no argument. After
  a 401 it is called once more with the refused token, so it can renew
  (`pk_med_smart::Renewer::after_refusal`). The request then goes once
  more with what it returns, unless that is the same token; a second 401
  is the answer (exit 3). An error from the source is the request's error,
  and nothing is sent. A fixed bearer is never renewed. The source should
  be cheap: keep the token in memory and go to the keychain only when it
  is due.
- Every request goes to the base's own origin; the bearer and extra headers
  (`header("Epic-Client-ID", …)`) never leave it. A `next` link or a Binary
  reference on another origin is refused (exit 5). The base must be `https`,
  or `http` to this machine for a test server.
- Ids and types are checked before any request (exit 2). An id is 1 to
  `MAX_ID_LEN` (256) of `A-Z a-z 0-9 - .`, and not only dots. FHIR R4
  allows 64, but an Epic app registered with "Use Unconstrained FHIR IDs",
  Epic's default, gets longer ones (80 and more), and Epic publishes no
  bound. `is_id` is that rule. A refused id is a usage error, the caller's
  input; a CLI holding an id from a server's response checks it with
  `is_id` first and decides what a refusal means there (an item skipped,
  exit 5). A path or a reference with a `.` or `..` segment, spelled out or percent-encoded, is
  refused (exit 2): a URL parser would climb out of the base.
- A 429, a 5xx, an edge's HTML 403, or a connection that failed is asked
  again under `RetryPolicy` (3 requests, 1 s then 2 s apart, or the
  server's `Retry-After`, 30 s at most; a server asking for longer is not
  waited for). `RetryPolicy` is `#[non_exhaustive]`: build it from
  `default()` or `none()` with its methods. A vendor's own retry guidance
  is the CLI's to set:

  | Method | Default | Set |
  |---|---|---|
  | `attempts(n)`, `delay(d)`, `max_wait(d)` | 3, 1 s, 30 s | requests in all, the first wait (doubled after), the longest wait |
  | `server_errors(&[502, 503, 504])` | every 5xx | only these 5xx are asked again; values outside 500–599 are ignored |
  | `retry_after_required(true)` | `false` | a 429 is asked again only with a `Retry-After` within `max_wait`, since one that does not say when may be a daily quota |
  | `jitter(0.25)` | `0` | each wait, a `Retry-After` included, lengthened by a random 0–25 %, never past `max_wait` |

  `on_retry` is told of each wait in one line (`the FHIR
  server at https://… answered HTTP 429 for Observation; asking again in
  5 s (2 of 3)`), so a CLI can say why it is quiet. The line names the
  origin, the resource type and the wait, never a query, an id or a token.
- A request that got no complete HTTP answer after its retries is exit 5
  with reason `pk_med_fhir::UNREACHABLE` (`Reason::Provider("unreachable")`):
  the connection failed, timed out, or broke before the whole answer was
  read. A server that answered with an error status has no such reason,
  and a reason the transport set is kept. cli-common's transport
  does not say which of those it was, so `UNREACHABLE` is not proof the
  network is gone: one slow page that timed out three times reads the
  same. A CLI that ends a run on it, rather than skip one type, can first
  ask the server something cheap (`get_raw("metadata")`) to tell the two
  apart.
- The bearer source and the retry observer are `Send + Sync`, so a client
  is `Send + Sync` whenever its transport is.
- A `next` link equal to the page just read ends the search with
  `truncated: true`, as the page cap does.

Refusals map to exit codes (`pk_med_fhir::outcome::status_error`):

| Answer | Exit |
|---|---|
| 401 | 3, `Reason::CredentialRefused` |
| 403 with FHIR or JSON | 3, `Reason::Forbidden` |
| 403 with an HTML page, 429, 5xx | 5, with the wait asked for |
| 404, 410 | 4 |
| 400, 422, any other 4xx | 5, `Reason::BadRequest` |
| a 3xx | 5: redirects are not followed |

An OperationOutcome in the body refines a 4xx by its first issue of
severity `error` or `fatal` (a warning never does), by the issue's code: `login`, `unknown`, `expired` give 3 (`CredentialRefused`);
`forbidden`, `suppressed`, `security` 3 (`Forbidden`); `not-found`,
`deleted` 4; `throttled`, `transient`, `timeout`, `exception` 5. An
OperationOutcome with errors in place of a resource with a 2xx is mapped the
same way. Messages name the server's origin and the resource type, never a
query, an id or a token, and quote the outcome's text cut to printable
characters. What to do next is the error's remedy, not part of its
message: `outcome::SIGN_IN_AGAIN` on exit 3 with `CredentialRefused`,
`outcome::TRY_AGAIN` on an answer that says nothing. A CLI replaces it with
its own (`err.with_remedy("run auth login --org …")`) instead of printing
two.

### 2.3 `test-util`: the mock server

Feature `test-util` adds `pk_med_fhir::test_util`: a loopback mock SMART +
FHIR server (`MockServer::start`) and the synthetic patient it serves, one
or more resources of every type above, a Binary carrying a one-page PDF,
and paging. It answers SMART discovery (both forms), the authorize step
(PKCE S256 required, `state` and `iss` returned, a declined consent on
request), the token endpoint (authorization code with the verifier checked,
rotating refresh, JWT bearer, an ID token for `openid` carrying the
authorization's `nonce`; `id_token_nonce_next` gives the next one another
or none) and registration. `Fault` makes chosen
requests fail: a 429 with `Retry-After`, an OperationOutcome error, a 401,
a 503. `withhold(type, text)` adds a `suppressed` warning with that text
to every page of the type's searches, in Epic's shape, its results served
as before. The authorize step takes a loopback redirect, and
`accept_redirect(uri)` adds a registered `https` page, which a test
follows as the page would, to the listener. Every value in it is synthetic: `crates/pk-med-fhir/fixtures/README.md`
lists the dummy shapes, and a test checks them positively.

`pk_med_records::test_util::write_synthetic_stream` writes that patient as
a complete stream, for a store's tests.

## 3. SMART App Launch (crate `pk-med-smart`)

A standalone patient launch (SMART App Launch 2.x) for a CLI, built on
cli-common's `pk_cli_auth::oauth` (PKCE S256, `state`, the loopback
redirect, RFC 9207 `iss`, the token and registration endpoints) and its
`RefreshLock`. No SMART or OAuth library of anyone else's.

What to do after an error is its remedy (`CliError::with_remedy`), never
part of its message, as in pk-med-fhir (§2.2). A CLI replaces it with its
own command and tells the remedies apart by `CliError::remedy`:

| Error | Remedy (`pk_med_smart::remedy`) |
|---|---|
| nothing stored for the organization and patient (exit 3, `NoCredential`) | `SIGN_IN` |
| a lapsed sign-in with no way to renew it, an ID token without this sign-in's nonce (exit 3, `CredentialRefused`) | `SIGN_IN_AGAIN` |
| a stored session that does not read (`Keychain`) | `CLEAR_AND_SIGN_IN_AGAIN` |
| a stored session in a newer format (`Keychain`) | `UPDATE` |
| a 429 or a 5xx for the SMART configuration or the CapabilityStatement (exit 5) | `TRY_AGAIN` |

`SIGN_IN_AGAIN` and `TRY_AGAIN` are spelled as `pk_med_fhir::outcome`'s, so
one match serves both crates.

### 3.1 The sign-in (`auth login`)

1. **Discovery.** `discover(transport, fhir_base, quirks, bin, version)`
   reads `<base>/.well-known/smart-configuration`. On a 3xx (redirects are
   not followed), a 4xx (not 429), or a document that does not read, it
   reads the `oauth-uris` extension of `<base>/metadata` instead. A 429 or
   5xx is exit 5 with no fallback: the server said nothing about where to
   sign in. Every URL the configuration keeps (the issuer and the
   authorization, token, registration, revocation and JWK Set endpoints)
   must be `https`, else exit 5. Plain `http` to this machine is allowed
   only when the FHIR base is itself `http` to this machine (the mock): an
   `https` server naming a local `http` endpoint would send the code, a
   client secret and the tokens to whatever local process holds the port
   (`SmartConfiguration::check_for`).
2. **Scopes.** `ScopeSet::standalone_patient()` is `openid fhirUser
   launch/patient offline_access`. Add `read_all(version)` (`patient/*.read`
   or `patient/*.rs`) or `read_types(version, &[types])` for a server that
   grants no wildcard. `ScopeVersion::for_server` picks v2 when the server
   lists `permission-v2`, else v1. `ScopeSet::parse` takes a vendor's own
   scope string as written.
3. **Authorize.** `begin(&SmartApp, LaunchRequest)` binds the listener on
   the first free registered port (none registered: any port) and builds the
   URL: PKCE S256 (a server that lists methods without S256 is refused),
   `state`, the scopes, and `aud`, which is the FHIR base with no trailing
   `/`, or `aud` when the server spells it differently. When the scopes ask
   for `openid`, it adds a fresh `nonce` (32 random bytes, base64url;
   athenahealth requires one with `openid`). When discovery named
   an issuer, an answer whose `iss` differs is refused (exit 3). An answer
   without `iss` passes (cli-common has no way to require it), and
   the `/metadata` fallback names no issuer at all, so there the RFC 9207
   check does not run. `SmartSignIn::issuer_checked()` says whether it can,
   and the session keeps the issuer it was bound to.

   `SmartApp::new(app, client, ports)` builds the app; it is
   `#[non_exhaustive]`, so a later field comes with a default. The redirect
   is the listener's own `http://127.0.0.1:<port>/callback` unless
   `.redirect_uri(uri)` names a registered `https` page, for a vendor that
   takes only `https` redirects in production (Epic: "An https protocol is
   required for use in a production environment"). The page sends the
   browser on to the listener with the query unchanged, so `state` and
   `iss` are checked as before, and the exchange names `uri` with the PKCE
   verifier, which never leaves the machine. The code passes through the
   page's host, whose logs can see it; it is single-use and useless without
   the verifier. The page forwards to one port, so the app names exactly
   that one. `begin` refuses (exit 2, from cli-common) a `uri` that is not
   `https`, has no host, names a user or carries a fragment, and any other
   number of ports. `SmartSignIn::redirect_uri()` is the redirect the
   address sent, and `listener_uri()` is where the answer must arrive.
4. **Exchange.** `SmartSignIn::wait_for_code(timeout, on_refused)`, then
   `exchange(oauth, client, code, organization, now)` returns a
   `SmartSession`. When a `nonce` was sent, an ID token that does not carry
   it, or carries another, is refused (exit 3, `CredentialRefused`), as
   OpenID Connect Core §3.1.3.7 requires: it answers some other sign-in.
   `SmartSignIn::nonce_checked()` says whether the check runs. The client
   is public (`ClientAuth::Public`) or
   confidential: `ClientAuth::SecretBasic` with the secret the CLI reads
   from its keychain (Kaiser's claims API), never from argv.
5. **Store.** Call `RefreshLock::before_signin` before step 3 and store the
   session inside `RefreshLock::store_signin`, so a sign-in never races a
   renewal, in the profile's `ProfileSessions` (§3.2).

**The window** (feature `browser`). `SignInWindow::open(bin, &profile,
&sign_in, before)` opens the address in Chrome in the profile's own kept
browser profile (cli-common `Launch::profile_name`:
`browser-profile-<profile>` under the CLI's data directory), so one login's
cookies never sign in another, and a provider that remembers a device
remembers it per login. `before` opens first, one tab each: a page a
provider needs signed in before its authorize address answers. Kaiser
Permanente's Epic organizations need none. Their 500 comes from the
capitalized `/<region>/Authentication/OAuth/Start` path, with or without a
kp.org session. The lowercase path, opened without cookies, reaches Kaiser's
own sign-in. mychart opens that one (its `epic::address_to_open`, 2026-11-19).
kp's authorize step, on `pfam.kp.org`, needs none either.
`open_at` takes a start page of the CLI's own instead, `https` or `http`
on `127.0.0.1`. Drop the window once the code arrives: Chrome is asked to
quit over its debug port, so the cookies it received are written and the
next sign-in in that profile finds them (a session that is still live goes
straight to consent). The system browser (`open_browser`) shares
one cookie jar across profiles, so a profile's sign-in does not use it.

`LaunchContext` reads SMART's launch context: `patient`, `encounter` and
`need_patient_banner` from the token response, and `fhirUser` from the
token response or the ID token. The ID token is read unverified and only
for display. The FHIR server, not this crate, decides access.

### 3.2 `SmartSession`: one vault entry per sign-in

A sign-in is named by `account(organization, patient)`,
`smart/<organization>/<patient>` (`_` for a sign-in with no patient in its
context). Each part has `%` and `/` percent-encoded (`a/b` is `a%2Fb`), so
two different pairs never name one session; a FHIR id has neither
character. The patient is the FHIR id, never the household's label for
them, so relabelling moves nothing.

**In a profile.** `ProfileSessions::new(&vault, profile)` is the
`SessionStore` a CLI uses: each session is the entry
`<profile>/<organization>/<patient>` in the CLI's vault (cli-common
`pk_cli_auth::vault::Vault::for_binary(bin)`: XChaCha20-Poly1305 files in
`~/.config/<bin>/vault/`, owner-only, under one keychain item). The whole
CLI reads that one item, the vault's key, however many profiles and
sign-ins it keeps, so a newly built or upgraded binary asks macOS once
rather than once per sign-in. `accounts()` lists the profile's sessions;
`delete_all()` is `profiles remove`'s. A CLI's other secrets (a
confidential client's secret) go in the same vault under a name with a
leading `_` (`_client-secret`), which no profile name has, so the CLI still
reads one item and a profile's listing never includes them. The account,
not the entry name, is the `session` a `PatientContext` names (§1.3); the
profile is the command's.

**Moving in.** `migrate_sessions(&CredentialStore::for_binary(bin),
&sessions, &accounts, &lock)` moves sessions kept the old way, one keychain
item at `account()` each, into a profile, under the refresh lock and with
no network. Per account, an old item the profile does not hold, or one
newer than the profile's copy by when its tokens were obtained, has its
text copied, read back, compared, and only then deleted, so a failure
leaves it for the next run; an older or unreadable one is deleted and the
profile's copy kept. It is idempotent, and a run with nothing left to move
only looks up absent items. `SessionStore` stays implemented for `CredentialStore` for that
reading. Each old item read by a newly built binary costs one macOS dialog,
once; test and sandbox sign-ins are deleted beforehand rather than moved,
with `security delete-generic-password -s piekstra.<bin> -a <account>`,
which, as an Apple tool, raises none. A downgrade past the move finds no
sign-in and means signing in again; that sign-in is newer than the moved
copy, so moving again after an upgrade keeps it.

**Prompts.** A binary signed without a team ID (`pk-cli-codesign`) is a new
code identity to the keychain at every build, so its first read of the
vault's key waits on one "Always Allow" dialog, and an unattended run
started before then waits on a dialog nobody answers. Installs and upgrades
on this Mac therefore happen only when the user is present, and the first
run after one is theirs.

`MemoryStore` serves tests, and `ProfileSessions` over a vault with
cli-common's `MemoryKeys` and a temporary directory. Every store is
`Send + Sync`, so it can sit behind a `FhirClient::bearer_with` source. A
session that does not read is an error naming a line and a column, never a
value; a vault error names an entry's file, which is named by a hash, and a
keychain error shows the account as `smart/<organization>/…`: never the
patient's id.

| Field | |
|---|---|
| `format` | the item's shape, `SESSION_FORMAT` (1). Every write stamps the writer's own format, whatever format the item was read with. A release refuses an item with a newer format ("written by a newer release of this CLI", remedy `UPDATE`) rather than read it in its own shape and write it back. An item without one is format 1 |
| `organization`, `patient`, `fhir_user` | who the sign-in is for |
| `fhir_base`, `issuer`, `token_endpoint` | where it was made; renewal reads these, not a fresh discovery |
| `client_id` | the client the tokens were issued to |
| `scope` | the scope asked for; the scope granted is `tokens.scope` |
| `tokens` | cli-common's `TokenSet`: access and refresh tokens, `expires_at`, the ID token, the answer's other members |
| `dynamic_client` | a client registered for this sign-in (§3.4) |
| `refused_at` | when the server last refused a grant for this sign-in (Unix seconds); cleared by the next grant it accepts (§3.3) |
| `extra` | a vendor's own state |

`Debug` on a session, its token set or its client shows no token or key.

### 3.3 Renewal

`Renewer::new(store, lock, oauth, client, quirks)`:

- `fresh(account, now)` returns the stored session. When its access token
  expires within 60 s, it first renews it under the refresh lock: it reads
  the item again, renews only if it is still due, and stores the result
  before releasing the lock. Two processes never spend one refresh token.
- `after_refusal(account, refused_access_token, now)` renews after a 401,
  unless another process already replaced the refused token.
- `renew(account, now)` renews now, due or not, unless another process
  renewed since it read the item: the first grant after a registration
  (§3.4).
- A refresh sends the scope granted (`tokens.scope`), or the scope asked
  for when the server named none: a refresh may ask only for what was
  granted (RFC 6749 §6), and a person can untick categories at consent.
  `VendorQuirks::refresh_scope` can change that, or send none.
- The renewer's `client` must be the one the tokens were issued to
  (`session.client_id`). Another client is refused before anything is
  sent, so one organization's secret never goes to another's server.
- A session from a registered client renews with the JWT-bearer grant
  instead. `Renewer::signer` takes a function that makes the signer from
  the `DynamicClient` read under the lock (`RsaKey::for_client` with
  feature `rsa`), so a sign-in another process stored meanwhile is signed
  with its own key. A signer whose `alg` or `kid` differs from the stored
  client's is refused before anything is sent.
- A grant refused with `Reason::CredentialRefused` (the token endpoint's
  `invalid_grant` or `invalid_client`, or nothing left to renew with)
  stamps `refused_at` on the stored session, under the lock, and keeps the
  rest. `fresh` treats a session with `refused_at` as lapsed, whatever its
  clock says: the refused token is not handed out again, the next use
  tries one grant, and a grant the server accepts clears it. With nothing
  to renew with, that use is refused (exit 3) before anything is sent. A
  status command shows a session with `refused_at` as refused, and a
  refusal the server takes back (one right after a registration, say)
  costs no new sign-in. The stamp is best effort: a keychain that refuses
  the write leaves it unset, and the refusal is still the error returned.

| Condition | Exit | `Reason` |
|---|---|---|
| nothing stored at the account | 3 | `NoCredential` |
| lapsed, and no refresh token or registered client | 3 | `CredentialRefused` |
| the renewer's client is not the session's, or the signer's key is not the registered one | 1 | |
| the token endpoint refuses | as `pk_cli_auth::oauth::refusal` maps it (`invalid_grant` is 3, `CredentialRefused`) | |

`fresh` is synchronous because the refresh lock is. The grant inside it
runs on `pk_cli_core::block_on`, so call it before the command's own
`block_on`, with a transport `block_on` can drive.

### 3.4 Dynamic client registration (Epic)

Epic's patient apps register a client per sign-in (RFC 7591) and get every
later access token with a JWT the client signs (RFC 7523). There is no
refresh token.

1. Sign in as in §3.1.
2. `register_dynamic_client(oauth, config, session, software_id, jwks,
   signer, key)` posts the registration with the exchange's access token as
   the initial access token. The metadata carries `software_id` (Epic: the
   app's client id), the public JWK Set, the JWT-bearer grant type, no
   response types, and `token_endpoint_auth_method: none`: the assertion is
   the grant (RFC 7523 §2.1), not a client authentication. It sends nothing
   else and returns the session with `dynamic_client` set and the
   exchange's tokens unchanged.
3. Store it (`RefreshLock::store_signin`), then `Renewer::renew` makes the
   first JWT-bearer grant under the lock. Registration and grant are two
   steps so that a grant that fails (a 5xx, a clock the server disputes)
   loses nothing: the client and its key are stored, the next `fresh` or
   `renew` tries again, and no second client is registered.
4. Each assertion has `iss` = `sub` = the registered client id, `aud` = the
   token endpoint, a random `jti`, and `exp` 4 minutes after `iat`
   (`DEFAULT_LIFETIME_SECS`), a minute inside Epic's 5-minute limit for a
   clock running ahead. The grant sends `client_id` and the scope
   `VendorQuirks::refresh_scope` gives (Epic documents none: a quirk
   returns `None`).

Signing goes through the `JwtSigner` trait (`alg`, `kid`, `sign`), so the
crate holds no key type by default. Feature `rsa` adds `RsaKey`: it
generates an RS384 key of at least 2048 bits from the system's random
source (Epic documents 2048), and provides `jwks()` (the public half only),
`kid()` (the RFC 7638 thumbprint), and `to_pkcs8()` / `from_pkcs8()` for
the keychain form that `DynamicClient` keeps. A stored key under 2048 bits
is refused on load, and the decoded key bytes are zeroed. It uses `rsa` 0.10, the
release that RUSTSEC-2023-0071 (Marvin) names as patched.

### 3.5 Vendor quirks

`VendorQuirks` collects a vendor's departures from the specification.
Every method defaults to SMART as written (`NoQuirks`).

| Method | Default | For |
|---|---|---|
| `discovery_headers` | none | Epic's `Epic-Client-ID` on discovery |
| `metadata_first` | `false` | a server whose `/metadata` is the reliable document |
| `authorize_params` | none | a tenant or practice selector |
| `scopes(requested)` | as requested | a server that refuses wildcards or spells scopes its own way |
| `refresh_scope(session)` | the scope granted, else the scope asked for | a server that refuses a scope on refresh or on the JWT-bearer grant (`None`) |
| `send_aud` | `true` | a server that refuses `aud` |
| `send_nonce` | `true` | a server that refuses `nonce`; nothing is checked then |

### 3.6 Scripted portal login: not used, and why

The patient's approval happens in a browser window the CLI opens (§3.1), where
the person signs in and approves. That covers the first grant and a new one
when a grant expires: about 90 days at athenahealth, up to a year at Epic.
Everything else runs on the stored session (§3.3), with no browser, password or
code. No CLI scripts a portal's sign-in or consent page, and none keeps a
portal password. That was the user's decision on 2026-11-17, from the findings below. A
design for scripted sign-in was drafted and closed unbuilt (#21).

The findings come from public pages read on 2026-11-12 to 2026-11-17 by
the provider CLIs' sessions:

| Vendor | Terms on scripted access | Protections on the sign-in page |
|---|---|---|
| athenahealth | forbidden, as the effort's review of its terms found | Okta Identity Engine at `identity.athenahealth.com` (`/.well-known/okta-organization`: `"pipeline": "idx"`) |
| Epic MyChart | no binding ban. Epic's App Developer Guidelines, which are advisory, ask that a developer "does not circumvent the display of any authentication or authorization mechanisms", and approving the consent page by script would. The MyChart mobile app licence bans "automated scripts or bots", for the app only | a `captchaRequired` flag; hCaptcha or reCAPTCHA where an organization turns one on |
| a MyChart organization's own terms | no automation clause in the one read; the organization may revoke access for any reason | as Epic MyChart |
| Kaiser Permanente (kp.org; Kaiser's Epic organizations sign in through Kaiser's own SAML sign-in) | forbidden on the plain reading: no "robot … or other automatic device, program, algorithm or methodology" may "access … any portion of the Site". Kaiser's Developer Portal terms take in kp.org's | PingOne Risk device profiling, reCAPTCHA wired in, F5; `robots.txt` disallows `/as/authorization.oauth2` |
| UnitedHealthcare (HealthSafe ID) | forbidden: "automated software, devices, scripts, robots", and circumventing a block | ThreatMetrix device profiling, reCAPTCHA |

### 3.7 Tests

The sign-in, renewal and registration are tested against §2.3's mock
server over the real HTTP transport, with a `MemoryStore`, and renewal
again in a profile's vault. The window's tests refuse a bad page before any
Chrome starts.

A CLI's own tests:

- **Unit tests** build the vault with cli-common's `MemoryKeys` and a
  temporary directory (`Vault::new`), and test the move with a second
  `MemoryKeys` as the old store: `migrate_sessions` takes any `KeyStore`
  as `from`. The moved, kept, newer-wins and failure paths are tested
  there.
- **Surface tests** run the binary with its `<PREFIX>_KEYCHAIN_SERVICE` test
  override (an empty service), so a logged-out command reads nothing: an
  empty vault reads no key. They never write the vault or the keychain.
  On macOS a write would leave a real item that every rebuilt test binary
  then prompts for. A surface test that points `HOME` at a temporary
  directory has no keychain at all (`CredentialStore` fails with "A
  default keychain could not be found"), so the moved path of
  `profiles migrate` is not reachable through the binary. It is the unit
  tests' to cover; the surface test checks the arguments and the
  nothing-to-move path.

## 4. DICOM headers (crate `pk-med-dicom`)

Our own reader of DICOM Part 10 files, enough to file a disc's studies and
no more: it reads the header and stops at Pixel Data. No pixel is decoded,
and no one else's DICOM library is used. flate2 inflates the deflated
transfer syntax.

### 4.1 What it reads

- The 128-byte preamble and `DICM`, or a file that starts with the file
  meta group and has no preamble.
- The file meta information (group 0002), explicit VR little endian, as
  PS3.10 requires.
- The data set in its transfer syntax:

  | Transfer syntax | UID |
  |---|---|
  | implicit VR little endian | `1.2.840.10008.1.2` |
  | explicit VR little endian | `1.2.840.10008.1.2.1` |
  | deflated explicit VR little endian | `1.2.840.10008.1.2.1.99` |
  | explicit VR big endian (retired) | `1.2.840.10008.1.2.2` |
  | any other (JPEG, RLE, ...): the data set is explicit VR little endian; only Pixel Data differs | |

- Sequences and items of defined or undefined length, nested up to
  `MAX_DEPTH` (16). A `UN` element of undefined length is read as an
  implicit VR little endian sequence (PS3.5 §6.2.2).
- Encapsulated pixel data inside a sequence (an icon) is skipped. Reading
  stops at the top-level Pixel Data tag (7FE0,0010). The reader buffers
  8 KiB, so it may hold up to that much past the tag, never decoded.
- An implicit-VR file names no VRs. A small dictionary gives the VR of each
  tag this crate reads by name; any other tag reads as `UN`, its bytes.

A hostile file cannot cost unbounded memory or time:

| Limit | Value | Past it |
|---|---|---|
| `MAX_VALUE_BYTES`, one text or number value | 16 MiB | refused (`DicomError::Limit`) before anything is allocated |
| the same, one bulk value (OB, OW, UN, ...) | 16 MiB | skipped (`Value::Skipped`), and reading goes on |
| `MAX_HEADER_BYTES`, what a header holds before Pixel Data: its values, and 128 bytes for each element and item | 64 MiB | refused |
| `MAX_INFLATED_BYTES`, a deflated data set inflated | 256 MiB | refused |
| `MAX_DEPTH`, sequence nesting | 16 | refused |

A skipped value is read past, never kept, so what it costs is bounded by
the file's own size, or by `MAX_INFLATED_BYTES` in a deflated one. A
File Meta Group Length that is negative or overflows is ignored, and the
meta group is read while its tags are group 0002.

### 4.2 `Header`

`read_header(reader)` (any `Read`, buffered inside) and `read_file(path)`
return a `Header`:

- `get(tag)` gives a text element's whole value: UTF-8 where it is valid,
  else Latin-1, without its padding, and several values keep their `\`. It
  is `None` when the element is absent, empty or not text. A tag is a `Tag`
  or `0xGGGGEEEE`. Group 0002 is read from the meta information, any other
  group from the data set's top level.
- `element(tag)` gives any element (sequences, numbers, bytes), and
  `meta()` and `dataset()` give the two data sets whole.
- Getters, each `None` when the file does not say:

| Getter | Type | Element |
|---|---|---|
| `study_uid`, `series_uid` | `&str` | Study and Series Instance UID |
| `sop_class_uid`, `sop_instance_uid` | `&str` | the data set's, else the meta information's |
| `modality`, `body_part` | `&str` | Modality, Body Part Examined |
| `study_date`, `series_date` | `String` | `YYYY-MM-DD` (`iso_date`): a day the calendar has, else `None` |
| `study_time`, `study_description`, `series_description` | `&str` | as written |
| `series_number`, `instance_number`, `rows`, `columns` | `i64` | |
| `number_of_frames` | `u32` | |
| `institution`, `accession` | `&str` | Institution Name, Accession Number |
| `referring_physician`, `patient_name` | `String` | `person_name`: `Prefix Given Middle Family Suffix`, from the alphabetic group |
| `patient_id`, `patient_sex` | `&str` | |
| `patient_birth_date` | `String` | `YYYY-MM-DD` |
| `transfer_syntax` | `&str` | from the meta information |
| `has_pixel_data` | `bool` | whether reading reached Pixel Data |

The patient getters are for the person's own terminal. `Debug` on a
`Header`, `Dataset`, `Element` or `Value` shows tags, VRs and sizes, never
a value, and an error names a byte offset and a tag, never a value.

### 4.3 DICOMDIR

`read_dicomdir(path)` lists every file a DICOMDIR names, as a
`DicomDirEntry { path, record_type, study_uid, series_uid, sop_uid,
modality }`. The study and series are those of the records above it.
`dicomdir_entries(header, root)` does the same for a header already read.

- Records are walked from the First Root Record through each record's
  Lower Level and Next offsets. Offsets count from the file's first byte,
  preamble included. An offset that points at no record, or a cycle, is
  refused (`DicomError::Malformed`). A record no offset reaches is left
  out. When the First Root Record is absent or zero, as some writers leave
  it, the records are read in sequence order instead, each under the last
  patient, study and series before it.
- A record whose Record In-use Flag is 0 is left out, as is one whose
  Referenced File ID is absent, empty or not text.
- Referenced File ID components (`IMAGES\S1\I1`) are joined under the
  DICOMDIR's directory. A component that is empty, `.` or `..`, absolute,
  or holds a `/` or `:` is refused (`DicomError::Malformed`), so a disc
  cannot name a file outside itself.

### 4.4 Summaries

`StudySummary::from_headers(&headers)` makes one summary per Study
Instance UID, in the order each study is first seen:

- the study's `date`, `description` and `accession_number`;
- `modalities`, sorted, each once;
- `series`, each a `SeriesSummary { uid, number, modality, description,
  body_part, instances }`, sorted by series number, with ties keeping the
  order first seen. `uid` is `None` for the files that name no Series
  Instance UID, grouped together;
- `instances`: distinct SOP Instance UIDs, so a second copy of a file
  counts once.

A file with no Study Instance UID is left out. A summary leaves out the
patient's name, id, birth date and sex. Its UIDs, accession number and date
still identify one visit, so it is medical data, for the person's own
terminal and records folder. It is not a wire shape: a CLI that prints one
wraps it in its own schema-tagged DTO.

### 4.5 Errors

| `DicomError` | `CliError` | Exit |
|---|---|---|
| `NotDicom`: no `DICM` and no meta group | `Usage` | 2 |
| `Truncated { offset }`, `Malformed { offset, what }`, `Limit { offset, what }`, `Io` | `Other` | 1 |

In a deflated data set, a stream that does not inflate is `Malformed`, one
that stops before its end is `Truncated` as a cut plain file is, and an
error of the file under it is `Io`, unchanged.

`DicomError` and `Value` are `#[non_exhaustive]`: a match needs a `_` arm,
so a later variant is not a break. `DicomError::Io` returns its
`io::Error` as `source()`, and `From<io::Error>` makes one.
`is_dicom(prefix)` tests a file's first 132 bytes, and `is_dicom_file(path)`
a file's, `false` when it does not open.

### 4.6 `test-util`: synthetic DICOM

`synthetic::SyntheticStudy::new(modality)` writes a study as a disc holds
it: `DICOMDIR` and `IMAGES/S<series>/I<instance>`. It returns the
instances' paths, series by series.

| Builder | Default |
|---|---|
| `series(n)`, `instances(n)` (each series) | 1, 3 |
| `seed(n)`: UIDs `2.25.0.<seed>`, `.<series>`, `.<instance>` | 1 |
| `study_date` (`YYYYMMDD` or `YYYY-MM-DD`) | 2026-03-04 |
| `patient_birth_date`, likewise | 1970-01-01 |
| `body_part`, `institution`, `accession`, `description` | `ABDOMEN`, `Example Imaging`, `ACC000001`, `Synthetic study` |
| `patient_name`, `patient_id` | `Testperson^Robin`, `PID000001` |
| `transfer_syntax(TransferSyntax::…)` | `ExplicitLittle` (also `ImplicitLittle`, `ExplicitBig`, `Deflated`) |
| `dicomdir(false)`, `dicomdir_without_offsets()` | a DICOMDIR with its offsets |

Each instance has:

- the modality's SOP class;
- Referring Physician `Clinician^Example`, and Series Description
  `Series <n>`;
- an undefined-length Referenced Study Sequence;
- an icon with encapsulated fragments;
- a 2×2 8-bit pixel matrix;
- an element after Pixel Data, which no reader may see.

The DICOMDIR is explicit VR little endian, with computed record offsets.
`encode`, `part10` and `Node` build any other data set a test needs.
Every value is a dummy, and `2.25.0` is no equipment's root: `2.25` is
the UUID arc, and no UUID is 0. No DICOM file is committed; tests write
them at run time.

## 5. Consumption

Pin a tag, never a branch or a path:

```toml
pk-med-records = { git = "https://github.com/piekstra/medical-common", tag = "v0.6.5" }
pk-med-fhir = { git = "https://github.com/piekstra/medical-common", tag = "v0.6.5" }
pk-med-smart = { git = "https://github.com/piekstra/medical-common", tag = "v0.6.5" }   # features = ["rsa"] for §3.4, ["browser"] for the window (§3.1)
pk-med-dicom = { git = "https://github.com/piekstra/medical-common", tag = "v0.6.5" }   # features = ["test-util"] for §4.6
```

Each medical-common tag pins one cli-common tag for every `pk-cli-*` crate,
and a consumer pins the same cli-common tag: two cli-common tags in one
build are two copies of every `pk-cli-core` type, and `CliError` from one
does not convert to the other.

| medical-common | cli-common |
|---|---|
| v0.1.0 | v0.31.0 |
| v0.2.0 | v0.31.1 |
| v0.3.0 | v0.31.1 |
| v0.4.0 | v0.31.1 |
| v0.4.1 | v0.31.1 |
| v0.4.2 | v0.31.1 |
| v0.4.3 | v0.31.1 |
| v0.4.4 | v0.31.1 |
| v0.4.5 | v0.31.1 |
| v0.4.6 | v0.31.1 |
| v0.4.7 | v0.31.1 |
| v0.4.8 | v0.31.1 |
| v0.4.9 | v0.31.1 |
| v0.5.0 | v0.32.0 |
| v0.6.0 | v0.33.0 |
| v0.6.1 | v0.33.1 |
| v0.6.2 | v0.33.2 |
| v0.6.3 | v0.33.2 |
| v0.6.4 | v0.33.2 |
| v0.6.5 | v0.33.2 |
