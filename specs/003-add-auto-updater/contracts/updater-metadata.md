# Contract: Stable Updater Metadata

## Endpoint

```text
https://github.com/msugenius/periscope/releases/latest/download/latest.json
```

The endpoint resolves only from the latest fully verified, published, non-prerelease SemVer release. Drafts and partial uploads must never be discoverable.

## JSON shape

```json
{
  "version": "0.2.0",
  "notes": "Generated release summary",
  "sourceCommit": "0123456789abcdef0123456789abcdef01234567",
  "platforms": {
    "windows-x86_64": {
      "signature": "<literal updater signature contents>",
      "url": "https://github.com/msugenius/periscope/releases/download/v0.2.0/periScope_0.2.0_x64-setup.exe"
    }
  }
}
```

## Validation

- `version` is strict stable `MAJOR.MINOR.PATCH` without a `v` prefix.
- `sourceCommit` is the 40-character lowercase SHA targeted by tag `v<version>`.
- Exactly one supported platform entry exists: `windows-x86_64`.
- `url` uses HTTPS, the expected repository, exact `v<version>` tag, and a safe base filename for the x64 NSIS installer.
- `signature` is the literal non-empty contents of the matching `.sig` file, never a path or URL.
- `notes` equals the release summary approved for the same release and is treated as untrusted display text.
- The installer name/version, release tag, metadata version, public manifest, and packaged application version agree.
- The release publisher verifies installer/signature/metadata bytes before marking the release latest.

## Version selection

- CD, not the client, guarantees that this document represents the greatest published stable SemVer.
- The client additionally requires the remote version to be strictly greater than its packaged version.
- Historical `release-<sha>` tags, drafts, prereleases, malformed versions, and releases without all required assets do not participate.
- The application does not enable downgrade or prerelease comparison overrides.

## Immutability and withdrawal

- The metadata endpoint may advance to a later release, but every package URL is pinned to its own version tag.
- Published release assets are never overwritten by CD.
- If a candidate is withdrawn before installation, package retrieval or verification fails safely; the client does not fall back to another package within the approved attempt.
