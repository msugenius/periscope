# Contract: Release Workflow

## Identity

- Workflow file: `.github/workflows/release.yml`
- Workflow display name: `Windows Release`
- Trigger: `pull_request` `closed`, base branch `master`
- Run condition: `github.event.pull_request.merged == true`
- Release concurrency group: `master-release`
- Queue policy: `queue: max`; never cancel an in-progress or pending release

## Trusted identity

Capture `github.event.pull_request.merge_commit_sha` as the full merge SHA. The following must derive from or equal it:

- Checkout ref
- Workflow artifact name
- Manifest `mergeSha`
- Git tag target
- Release target commit
- Deterministic tag `release-<full-merge-sha>`

Do not resolve `master` at build time; it may have advanced.

## Build job

- Runner: `windows-2025`
- Permissions: `contents: read`, all others `none`
- Inputs: exact merged source, locked dependencies, pinned toolchains
- Build: x64 Tauri NSIS only
- Outputs: one `*-setup.exe` and `release-manifest.json`
- Verification: installer exists, is non-empty, matches expected architecture/type, and has recorded SHA-256 and size
- Handoff: short-lived workflow artifact named `windows-<full-merge-sha>`

A build failure, cancellation, timeout, or missing output prevents the publisher job.

## Publisher job

- Depends on successful build job
- Does not check out or execute application source
- Permissions: `contents: write`, all others `none`
- Uses the built-in `GITHUB_TOKEN`; no personal token
- Downloads only the exact workflow artifact from its dependency
- Validates the manifest according to [release-manifest.md](./release-manifest.md)

## Publication state machine

1. Derive deterministic tag and inspect existing tag/release state.
2. If an existing published release targets the same SHA and its assets match the manifest, return success without mutation.
3. If an existing tag/release conflicts with the SHA or verified asset, fail.
4. Create or resume a matching draft release.
5. Determine the immediately previous published release tag, if one exists.
6. Generate release notes from that tag to the current merge target; omit the start tag for the first release.
7. Upload the installer and manifest to the draft.
8. Verify release target, asset names, sizes, and digests.
9. Publish the draft as the final mutating step.

Never replace or delete an asset from an already verified public release.

## Release presentation

- Tag: `release-<full-merge-sha>`
- Title: `periScope <application-version> (PR #<number>, <short-sha>)`
- Notes: GitHub-generated change list, contributors, and full comparison link
- Assets: one x64 NSIS installer and one release manifest
- Draft: false only after all verification passes
- Prerelease: false

## Failure and retry behavior

- A failure before draft creation leaves no release.
- A failure after draft creation leaves a non-public draft that a retry may resume.
- A retry for an already correct public release is a no-op success.
- A mismatch is an actionable hard failure, not an overwrite.
- Serialized queued runs preserve prior-release ordering for changelog generation.
- If more than 100 release runs are simultaneously pending, GitHub's platform queue limit is reached and maintainers must rerun any rejected merge release; this operational limit must be visible in run diagnostics.

## Performance and retention

- Timeout: 20 minutes per normal release run.
- Cache only dependency/build inputs needed to meet the budget; no credentials.
- Workflow artifacts use short retention.
- Public installer and manifest remain with the GitHub release.
