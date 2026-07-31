# Data Model: Automatic Application Updates

The updater persists no new user data. These models describe process memory, release metadata, and workflow handoff/publication records.

## Installed Version

Version identity of the running application.

| Field | Type | Rules |
|-------|------|-------|
| `version` | Stable semantic version | Exact `MAJOR.MINOR.PATCH`; sourced from packaged application metadata |
| `platform` | Platform identifier | `windows-x86_64` for this feature |

## Release Candidate

Safe serializable projection of one plugin-provided update offered to the user.

| Field | Type | Rules |
|-------|------|-------|
| `version` | Stable semantic version | Strictly greater than Installed Version; no prerelease/build suffix |
| `notes` | Text | Release summary; treated as untrusted text by the UI |
| `sourceCommit` | Full commit SHA | 40 lowercase hexadecimal characters from metadata |
| `platform` | Platform identifier | Must equal `windows-x86_64` |

The native updater retains the corresponding non-serializable plugin update object separately. That object is the authority for URL, signature, and download/install behavior; the displayed version must match it before installation begins.

## Update Session

Single process-scoped state exposed to each settings webview.

| Field | Type | Rules |
|-------|------|-------|
| `phase` | Update Phase | Follows transitions below |
| `installedVersion` | Stable semantic version | Always present |
| `candidate` | Release Candidate or absent | Present only when an update is pinned or actively processed |
| `downloadedBytes` | Non-negative integer or absent | Present during download when known; never exceeds total |
| `totalBytes` | Positive integer or absent | Optional when the server does not provide a length |
| `failureCode` | Stable code or absent | Present only in `failed`; no secrets or raw URLs |
| `message` | Safe display text or absent | Human-readable, escaped by UI |

### Update phases

```text
idle
  → checking
      → up-to-date
      → available
      → failed

available
  → dismissed
  → downloading

downloading
  → installing
  → failed

installing
  → process exits and updated installer restarts application
  → failed
```

Terminal-for-session states are `up-to-date` and `dismissed`. `failed` may expose an explicit retry action only when no install mutation has started. A new webview reads the existing state; it does not reset it to `idle`.

### Invariants

- At most one automatic `idle → checking` transition occurs per process.
- At most one candidate is retained.
- Only `available` may begin download.
- Install input version must equal the retained candidate version.
- No mutex is held across network, download, or installer awaits.
- Dismissal and terminal failure drop the pending plugin update object.
- Progress events are monotonic within one attempt.
- A successful Windows install exits the process; there is no in-process `installed` state.

## Release Version Declaration

Reviewed version carried by merged source.

| Field | Type | Rules |
|-------|------|-------|
| `authoritativeVersion` | Stable semantic version | Read from `src-tauri/tauri.conf.json` |
| `cargoVersion` | Stable semantic version | Must equal authoritative version |
| `cargoLockVersion` | Stable semantic version | periScope lock entry must equal authoritative version |
| `npmVersion` | Stable semantic version | Must equal authoritative version |
| `npmLockVersion` | Stable semantic version | Root lock entry must equal authoritative version |
| `tag` | String | Exactly `v<authoritativeVersion>` |
| `mergeSha` | Full commit SHA | Exact merged PR commit built and tagged |

### Validation

- Syntax is exactly three dot-separated numeric identifiers.
- Numeric identifiers have no leading zero unless equal to zero.
- Prerelease and build metadata are not accepted by this stable channel.
- All declarations and lock mirrors agree.
- Version is greater than the migration floor and every published stable SemVer tag.
- A reused tag is valid only for a verified retry targeting the same merge SHA.

## Private Release Handoff

Deterministic workflow artifact produced by the read-only build/sign job.

| Field | Type | Rules |
|-------|------|-------|
| `schemaVersion` | String | `2` |
| `repository` | `owner/name` | Must match trusted workflow context |
| `mergeSha` | Full commit SHA | Must match event and checkout |
| `version` | Stable semantic version | Must match declarations |
| `tag` | String | `v<version>` |
| `installer` | File Evidence | Exact x64 NSIS installer |
| `signature` | File Evidence | Exact matching updater `.sig` file |

`File Evidence` contains a base filename, positive byte size, and lowercase SHA-256. It contains no build time, run ID, absolute path, token, or secret.

## Updater Metadata

Public `latest.json` consumed by installed applications.

| Field | Type | Rules |
|-------|------|-------|
| `version` | Stable semantic version | No `v` prefix; equals release version |
| `notes` | Text | Generated release summary |
| `sourceCommit` | Full commit SHA | Equals tag/release target |
| `platforms.windows-x86_64.url` | HTTPS URL | Immutable asset URL under `v<version>` |
| `platforms.windows-x86_64.signature` | Signature text | Literal `.sig` contents, not a URL |

## Public Release Manifest

Deterministic traceability record generated by the publisher after `latest.json` exists.

| Field | Type | Rules |
|-------|------|-------|
| `schemaVersion` | String | `2` |
| `repository` | `owner/name` | Project repository |
| `mergeSha` | Full commit SHA | Release/tag target |
| `version` | Stable semantic version | Equals metadata/package version |
| `tag` | String | `v<version>` |
| `installer` | File Evidence | Public installer evidence |
| `signature` | File Evidence | Public `.sig` evidence |
| `updaterMetadata` | File Evidence | Public `latest.json` evidence |

The manifest does not hash itself. GitHub provides publication time and public asset identity around this deterministic content.

## Relationships

- One Installed Version has zero or one Release Candidate per process.
- One Update Session owns at most one retained plugin update object corresponding to its candidate.
- One Release Version Declaration produces one Private Release Handoff.
- One valid handoff produces one Updater Metadata document and one Public Release Manifest.
- One published release owns exactly one installer, one signature, one updater metadata document, and one public manifest.
- The release tag, source commit, application version, candidate version, package URL, and both manifests must form one consistent identity chain.
