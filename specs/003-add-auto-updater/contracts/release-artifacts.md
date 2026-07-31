# Contract: SemVer Release and Updater Artifacts

## Release identity

- Trigger: successfully merged pull request into `master`
- Source: exact `github.event.pull_request.merge_commit_sha`
- Version authority: `src-tauri/tauri.conf.json`
- Required mirrors: `src-tauri/Cargo.toml`, periScope's `src-tauri/Cargo.lock` entry, `package.json`, and root `package-lock.json` entry
- Tag: `vMAJOR.MINOR.PATCH`
- Title: `periScope MAJOR.MINOR.PATCH (PR #<number>, <short-sha>)`
- State: draft until all four public assets and tag target pass verification; then published, non-prerelease, and latest

All version declarations are strict, equal, and greater than the greatest earlier published stable SemVer. The first SemVer publication must exceed the `0.1.0` migration floor.

## Private handoff

The trusted build/sign job uploads a short-lived workflow artifact containing:

1. One `periScope_<version>_x64-setup.exe` installer.
2. Its matching updater `.exe.sig` file.
3. `release-handoff.json` with deterministic identity, names, sizes, and SHA-256 digests.

The publisher treats every handoff field as untrusted and verifies repository/SHA/version against workflow context, filenames against safe base-name patterns, and file evidence against downloaded bytes.

## Public assets

Exactly four assets are published:

1. `periScope_<version>_x64-setup.exe`
2. Matching updater signature
3. `latest.json` conforming to [updater-metadata.md](./updater-metadata.md)
4. `release-manifest.json`

The public manifest contains schema `2`, repository, merge SHA, version, tag, and name/size/SHA-256 evidence for the other three assets. It contains no timestamp, workflow run ID, secret, absolute path, or digest of itself.

## Publication state machine

```text
triggered
  → version-validated
  → existing-state-inspected
      → already-published-and-verified (successful no-op)
      → source-checked-out
          → built-and-signed
          → handoff-verified
          → draft-created-or-resumed
          → metadata-and-manifest-generated
          → assets-uploaded-and-verified
          → published-and-marked-latest
```

Any mismatch transitions to `failed`; CD does not overwrite, delete, or replace a public asset.

## Retry and conflict rules

- Same version, same merge SHA, and four matching public assets: successful no-op before rebuilding.
- Same version or tag targeting another SHA: hard failure.
- Matching draft and SHA: resume only if every existing asset matches or is absent.
- Conflicting draft asset, unexpected asset, missing signature, invalid manifest, or malformed metadata: hard failure.
- Failed build/sign: publisher does not start.
- Failed publisher: draft remains non-public for a verified retry.
- Version equal to or below the greatest earlier stable SemVer: fail before build.

## Changelog ordering

- For later SemVer releases, generated notes start from the greatest earlier published stable SemVer tag.
- For the first SemVer release, the latest published legacy `release-<sha>` tag may be used only as the changelog predecessor.
- Release ordering uses numeric SemVer, not lexicographic tag text or publication timestamp.

## Security boundary

- Quality/PR jobs receive no signing or publishing secret.
- Build/sign job runs only trusted merged source, has `contents: read`, and receives the updater private key only for the signing step or protected release environment.
- Private key and password never enter artifacts, manifests, caches, command echo, or logs.
- Publisher receives `contents: write`, does not check out or execute application source, and never receives the updater private key.
- Only the public updater key is committed to the application configuration.
