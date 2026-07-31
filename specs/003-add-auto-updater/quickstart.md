# Quickstart: Validate Automatic Application Updates

This guide validates the planned implementation locally and through controlled Windows/GitHub scenarios. Never use the production updater private key for local tests.

## Prerequisites

- Windows x64 with Tauri's Microsoft C++ build prerequisites and WebView2
- Node.js 24 and npm
- Rust 1.97.0 with the repository's pinned components
- `cargo-llvm-cov`
- GitHub CLI for controlled release inspection
- A disposable updater signing keypair and disposable test repository/environment for installation tests

Install locked dependencies from the repository root:

```powershell
npm ci
rustup show
cargo llvm-cov --version
```

Expected: npm uses `package-lock.json`, Rust 1.97.0 and the x64 Windows target are active, and the coverage tool reports its version.

## 1. Validate local quality and release rules

```powershell
npm run format:check
npm run lint
npm run test:coverage
pwsh -File scripts/release/Test-ReleasePipeline.ps1
```

Expected:

- Frontend and Rust tests pass with at least 80% line coverage independently.
- Updater state, UI, failures, and version precedence tests pass.
- Release-helper tests accept numeric increases such as `1.9.0 → 1.10.0` and reject mismatches, leading zeroes, prerelease/build suffixes, equal/lower versions, and reused conflicting tags.

## 2. Validate version declarations

Inspect these declarations in one release-ready change:

- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- periScope entry in `src-tauri/Cargo.lock`
- `package.json`
- root entry in `package-lock.json`

Expected: all contain one identical strict `MAJOR.MINOR.PATCH` version. For the updater feature, the expected version is `0.2.0` if the stable floor remains `0.1.0` at merge time.

Change one declaration on a disposable branch and rerun the release-helper test. Expected: validation identifies the mismatched file and exits nonzero before any build.

## 3. Build signed updater artifacts with a disposable key

Generate a non-production keypair using the official Tauri signer. Store the private key outside the repository and expose it only in the current disposable shell according to the updater documentation; do not print it.

Run the existing NSIS build target:

```powershell
npm run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis
```

Expected:

- Exactly one versioned x64 NSIS installer and its updater signature are produced.
- The signature file is non-empty and the private key does not appear in source, build output, logs, or caches.
- No JavaScript updater/process production dependency was added.

## 4. Validate non-blocking update UI behavior

The webview uses only the custom native commands `get_update_status`,
`start_update_check`, `dismiss_update`, and `install_update`, plus the
`periscope://updater-state` full-snapshot event. To use a controlled endpoint,
change only `plugins.updater.endpoints` in a disposable test build's
`src-tauri/tauri.conf.json`; never point a production-signed build at a test
server or reuse the production private key.

Run the automated UI/native tests with mocked boundary outcomes for:

- Newer stable version available
- Equal, lower, prerelease, malformed, or incompatible version
- Offline, timeout, malformed metadata, and invalid signature
- Dismissal, duplicate action, progress, download failure, and install failure
- Settings webview destruction/recreation during a check or offer

Expected:

- Normal Settings and crosshair behavior is immediately usable.
- One native check occurs per process even across recreated webviews.
- Offered version and escaped notes are clear; dismissal lasts for the process session.
- A pinned version mismatch is rejected and no raw diagnostic or secret reaches the UI.

Compare snapshots and transitions with [contracts/updater-ipc.md](./contracts/updater-ipc.md).

Automated evidence: [US1 detection](./validation/us1-detection.md) and
[US2 installation logic](./validation/us2-automated.md). Real installer evidence
belongs in `validation/us2-installation.md` after the isolated Windows exercise.

## 5. Validate a controlled GitHub release

In a disposable repository configured like production:

1. Install the disposable updater public key in the test build and configure its test release endpoint.
2. Merge a fully passing release-ready PR into the protected release branch.
3. Verify the workflow builds the exact merge SHA and uses the private key only in the trusted sign step.
4. Inspect the draft before publication and the final release afterward.

Expected:

- Tag is `v<version>` and targets the exact merge SHA.
- The release contains exactly the installer, signature, `latest.json`, and public manifest.
- Metadata uses an immutable version-tagged installer URL and embeds literal signature contents.
- Names, versions, SHA, sizes, and digests agree with [contracts/release-artifacts.md](./contracts/release-artifacts.md).
- The release becomes public/latest only after all verification succeeds.

## 6. Validate release failures and retries

Use controlled/disposable release state to exercise:

1. Missing or invalid signing secret.
2. Version equal to or lower than the greatest published stable SemVer.
3. Same tag targeting another commit.
4. Matching draft with missing assets.
5. Draft with a conflicting asset digest.
6. Completed workflow rerun for the same version/SHA.
7. Two sequential queued releases with increasing versions.

Expected:

- Build/sign failures never start the publisher.
- Conflicts fail without overwriting assets.
- A valid draft resumes; a complete public release is verified and returns success before rebuilding.
- Only increasing versions publish, and changelog ordering follows numeric SemVer.

The pure release-rule evidence is produced by
`scripts/release/Test-ReleasePipeline.ps1`. Disposable GitHub publication and
contract comparisons belong in `validation/us3-release.md` and
`validation/us3-contract.md`; do not substitute local mocks for those records.

## 7. Validate real Windows replacement

Use a disposable Windows VM or equivalent isolated installation, not the developer installation:

1. Publish two genuinely built and updater-signed test versions.
2. Install the older version and save non-default settings.
3. Launch while the newer stable release is latest.
4. Confirm the offer, approve it, observe progress, and allow passive installation/restart.
5. Repeat with a corrupted installer/signature and with permission or storage failure where the environment can simulate them safely.

Expected:

- The successful restart reports the approved newer version and preserves settings.
- Corruption or signature mismatch is rejected.
- Failed installation leaves the older version runnable with a retry path.
- Declining before download is the supported cancellation path; mid-download cancellation is outside this feature.

## 8. Record performance and footprint evidence

Measure representative cold and warm launches/checks and record:

- Time to first interactive settings render with and without updater
- Update result latency under normal network conditions
- Network request count per process
- Idle CPU and retained updater memory after completion/dismissal
- Installed/installer size before and after the feature

Expected:

- No more than 100 ms p95 added interactive startup time.
- At least 95% of normal checks resolve within 10 seconds.
- Exactly one automatic request sequence per process and no periodic polling.
- No measurable post-check idle CPU regression.
- Installed footprint grows by no more than 2 MiB relative to the recorded 1,968,509-byte NSIS baseline.
