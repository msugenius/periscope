# Data Model: CI/CD and Windows Releases

The feature adds no application-persisted domain data. These entities describe records owned by GitHub Actions and GitHub Releases and the manifest passed between the isolated build and publication jobs.

## Validation Run

Represents one quality evaluation for a pushed commit or eligible pull-request revision.

### Fields

| Field | Type | Rules |
|-------|------|-------|
| `runId` | Service identifier | Unique and immutable |
| `triggerKind` | `branch-push` or `pull-request` | Derived from the event |
| `sourceSha` | Full commit SHA | Required; immutable |
| `sourceRef` | Branch/ref name | Required for pushes |
| `pullRequestNumber` | Positive integer or absent | Required for PR runs |
| `targetBranch` | Branch name or absent | `dev` or `master` for eligible PR runs |
| `formatStatus` | Stage status | Always present |
| `lintStatus` | Stage status | Runs only after format passes |
| `testStatus` | Stage status | Required only for eligible PR runs |
| `frontendLineCoverage` | Percentage or absent | Required after frontend PR tests |
| `rustLineCoverage` | Percentage or absent | Required after Rust PR tests |
| `startedAt` / `completedAt` | Timestamps | Used for duration budgets |
| `failureStage` | Stage name or absent | Set when overall result fails |

### Stage states

`pending → running → passed`

`pending → running → failed`

`pending → skipped` when an earlier stage fails or the stage does not apply.

`pending|running → cancelled` when a newer run supersedes the same branch/PR revision.

### Validation rules

- Lint cannot start until format is `passed`.
- Tests cannot start until lint is `passed`.
- A branch-push run has `testStatus = skipped`.
- A PR run targeting `dev` or `master` passes only when both coverage percentages are at least 80.
- Overall success requires every applicable stage to pass.

## Release Run

Represents processing initiated by one successfully merged pull request into `master`.

### Fields

| Field | Type | Rules |
|-------|------|-------|
| `runId` | Service identifier | Unique |
| `pullRequestNumber` | Positive integer | Required |
| `mergeSha` | Full commit SHA | Required; authoritative source identity |
| `sourceBranch` | Branch name | Traceability only |
| `applicationVersion` | Project version string | Read from merged source |
| `releaseTag` | String | Exactly `release-<mergeSha>` |
| `workflowArtifactName` | String | Includes full merge SHA |
| `state` | Release-run state | Follows transition rules below |
| `previousReleaseTag` | String or absent | Absent only for first publication |
| `failureStage` | Stage name or absent | Set on failure |
| `startedAt` / `completedAt` | Timestamps | Used for duration budgets |

### State transitions

```text
triggered
  → source-checked-out
  → built
  → artifact-verified
  → draft-created-or-resumed
  → assets-uploaded
  → publication-verified
  → published
```

Any non-terminal state may transition to `failed` or `cancelled`. A retry resumes from verified external state. If the deterministic release is already published and matches the manifest, the retry transitions directly to `published`.

### Validation rules

- The event must be a merged PR with `master` as target.
- Checkout SHA, manifest SHA, tag target, and release target must all equal `mergeSha`.
- Publication cannot begin before the build artifact and manifest pass verification.
- At most one published release may use `releaseTag`.
- A conflicting existing tag, commit, asset name, or digest is a hard failure.

## Release Manifest

Immutable JSON handoff produced by the read-only build job and consumed by the publisher.

### Fields

| Field | Type | Rules |
|-------|------|-------|
| `schemaVersion` | String | Initially `1` |
| `repository` | `owner/name` | Must equal the workflow repository |
| `pullRequestNumber` | Positive integer | Must match event |
| `mergeSha` | 40-character commit SHA | Must match checked-out source |
| `applicationVersion` | String | Must match build metadata |
| `artifactName` | File name | One x64 NSIS `*-setup.exe` |
| `artifactSha256` | 64-character lowercase hex | Recomputed by publisher |
| `artifactSizeBytes` | Positive integer | Recomputed by publisher |
| `builtAt` | UTC timestamp | Informational |
| `workflowRunId` | Service identifier | Traceability |

## Release

User-visible GitHub publication for one merge.

### Fields

| Field | Type | Rules |
|-------|------|-------|
| `tag` | String | Equals manifest-derived deterministic tag |
| `title` | String | Includes app version, PR number, short SHA |
| `targetCommitish` | Full commit SHA | Equals manifest `mergeSha` |
| `draft` | Boolean | True until all verification passes |
| `publishedAt` | Timestamp or absent | Present only after publication |
| `installer` | Release asset | Name, size, and digest match manifest |
| `manifest` | Release asset | Exact build-job handoff |
| `changelog` | Generated notes | Covers prior published tag to current target |

## Changelog Entry

Traceable summary supplied by GitHub-generated release notes.

### Fields

| Field | Type | Rules |
|-------|------|-------|
| `changeIdentifier` | Pull request/commit reference | Present when source metadata exists |
| `title` | Text | Derived from repository metadata |
| `contributor` | User reference | Present when GitHub metadata exists |
| `comparisonRange` | Previous/current tag references | Full range shown once per release |

## Relationships

- One pull-request revision has one current **Validation Run** and may have superseded earlier runs.
- One successfully merged `master` pull request initiates one logical **Release Run**.
- One **Release Run** produces one **Release Manifest** and one **Release**.
- One **Release** owns exactly one NSIS installer, one manifest, and many **Changelog Entries**.
- A release's `previousReleaseTag` references the preceding published **Release**, forming the changelog sequence.
