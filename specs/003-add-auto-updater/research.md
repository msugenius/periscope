# Research: Automatic Application Updates

## Decision 1: Use the official Tauri updater from native Rust

**Decision**: Add the Windows Rust updater plugin and expose project-specific commands/events rather than calling an updater JavaScript package directly.

**Rationale**: The plugin supplies SemVer comparison, signed download verification, passive NSIS installation, and restart behavior. periScope destroys the settings webview when it is hidden and creates a new one later, so a JavaScript-only guard would repeat checks and lose the approved candidate. Native process state survives that lifecycle and avoids JavaScript updater/process dependencies and capability permissions.

**Alternatives considered**:

- Direct GitHub release calls plus a custom installer: rejected because they duplicate signed update, comparison, and replacement behavior.
- JavaScript updater and process plugins: rejected because state would be coupled to a disposable webview and adds dependencies without value.
- Periodic polling: rejected because once-per-launch checking meets the requirement without idle work.

**Sources**: [Tauri updater guide](https://v2.tauri.app/plugin/updater/), [official updater implementation](https://raw.githubusercontent.com/tauri-apps/plugins-workspace/v2/plugins/updater/src/updater.rs)

## Decision 2: Use a static GitHub release manifest

**Decision**: Configure `https://github.com/msugenius/periscope/releases/latest/download/latest.json` as the single stable endpoint. Each metadata document points to an immutable `releases/download/v<version>/<installer>` URL.

**Rationale**: GitHub remains the source requested by the feature, while a static manifest gives the updater one small machine-readable document. CD guarantees that the release marked latest is the greatest valid stable SemVer; the version-tagged package URL keeps an approved candidate pinned even if a newer release appears.

**Alternatives considered**:

- Scan every GitHub release in the application: rejected because it increases request count, rate-limit exposure, parsing code, and dependency on hosting API behavior.
- Point package URLs through `/latest/download`: rejected because the asset could change between approval and installation.
- Run a separate update service: rejected because GitHub release assets already satisfy the need.

**Sources**: [Tauri static manifest](https://v2.tauri.app/plugin/updater/#static-json-file), [GitHub latest release semantics](https://docs.github.com/en/rest/releases/releases#get-the-latest-release)

## Decision 3: Persist update state for the native process lifetime

**Decision**: Store phase, displayed candidate, progress, safe failure, and the exact pending updater object in a dedicated managed native state. A small snapshot/event contract serves every settings webview.

**Rationale**: `hide_settings` closes the current webview and `show_settings` builds a new one. Native ownership enforces one automatic check per process, prevents duplicate installs, preserves progress, and releases a dismissed pending resource. The exact updater object pins version, URL, signature, and raw metadata approved by the user.

**Alternatives considered**:

- Module globals in `app.ts`: rejected because they disappear with the webview.
- Persistent disk storage: rejected because update state is session-only and stale candidates must be rechecked next launch.
- Put updater fields into general settings state: rejected because user settings and transient update operations have different lifetimes and responsibilities.

## Decision 4: Use passive signed NSIS installation

**Decision**: Generate updater artifacts, embed the public updater key, keep the private key in release-only secrets, and use the passive Windows install mode.

**Rationale**: Tauri updater signatures are mandatory authenticity evidence; SHA-256 alone only detects accidental or untrusted modification after a trusted digest is known. Passive installation shows progress, supports elevation, and restarts on Windows without another process plugin. The app exits through the updater's installer path, so the existing close-to-tray handler does not block replacement.

**Alternatives considered**:

- Quiet mode: rejected because it cannot request elevation unless periScope is already elevated.
- Authenticode alone: rejected because Windows code signing and updater signing solve different trust problems.
- Silent background update: rejected because the specification requires explicit approval.

**Sources**: [Updater signing and Windows modes](https://v2.tauri.app/plugin/updater/#signing-updates), [Windows code signing](https://v2.tauri.app/distribute/sign/windows/)

## Decision 5: Keep strict source-declared stable SemVer

**Decision**: Publish only `vMAJOR.MINOR.PATCH` releases. Treat `tauri.conf.json` as the application version authority and require its value to match Cargo, npm, and both lockfile declarations before quality or release processing succeeds.

**Rationale**: Source-declared versions make the built commit reproducible and reviewable. Validated mirrors fit the existing repository with less structural change than adding a new workspace/version tool. Strict stable syntax excludes prerelease/build channels from the first implementation.

**Alternatives considered**:

- Infer bumps from commit messages: rejected because it introduces policy and failure cases the user did not request.
- Increment during CD: rejected because the built version would differ from merged source.
- Keep `release-<sha>` tags: rejected because installed clients need ordered semantic identities.
- Add a new versioning dependency or service: rejected because strict three-part numeric comparison is small and testable in the release helper.

**Sources**: [Semantic Versioning 2.0.0](https://semver.org/)

## Decision 6: Extend the verified two-job release boundary

**Decision**: Retain the read-only build/sign job and write-only publisher, but extract pure release rules into a tested PowerShell module. The build job performs an early published-retry check, signs one installer, and hands off deterministic files. The publisher generates metadata, verifies a draft, and publishes last.

**Rationale**: The current workflow already protects source execution from the write token and prevents partial public releases. A helper module makes new SemVer, filtering, manifest, and conflict rules directly testable. Early retry detection avoids rebuilding a potentially byte-different installer and fixes current manifest idempotency problems caused by timestamps/run IDs.

**Alternatives considered**:

- Let `tauri-action` own release creation: rejected because it collapses the established least-privilege handoff and custom verification.
- Keep all logic inline in YAML: rejected because the existing workflow is already large and new pure rules require tests.
- Rebuild and overwrite on retry: rejected because published artifacts must be immutable and reproducible identity does not imply byte-identical installer output.

**Sources**: [Tauri Action](https://github.com/tauri-apps/tauri-action), [GitHub generated release notes](https://docs.github.com/en/rest/releases/releases#generate-release-notes-content-for-a-release)

## Decision 7: Publish four verified public assets

**Decision**: Each stable release publishes the normal NSIS installer, its updater `.sig`, `latest.json`, and a deterministic `release-manifest.json`. A separate deterministic handoff manifest remains private to the workflow.

**Rationale**: The updater needs the installer and literal signature from `latest.json`; users need the normal installer; maintainers need traceability and digests. Separating private handoff authorization from public release evidence avoids treating build-produced metadata as trusted and avoids a manifest self-hash cycle.

**Alternatives considered**:

- Publish only installer and SHA manifest: rejected because the updater requires a cryptographic signature and static endpoint metadata.
- Publish the private handoff unchanged: rejected because `latest.json` and its digest do not exist until the publisher has release notes and final URLs.
- Include timestamps/run IDs in byte identity: rejected because those make valid retries conflict.

## Decision 8: Split automated and controlled verification at real boundaries

**Decision**: Unit-test all owned version, state, UI, metadata, and failure logic; use a disposable Windows environment with two signed builds for actual installer replacement, restart, elevation, disk, and settings-preservation evidence.

**Rationale**: Browser/jsdom mocks do not run the Rust backend, and the native updater has no mid-download abort signal. Real installer behavior cannot be proven safely by overwriting a developer installation. Declining before download is the supported cancellation path; custom cancellable downloading is outside this feature.

**Alternatives considered**:

- Treat mocked frontend tests as end-to-end evidence: rejected because they do not exercise signatures, process exit, or NSIS.
- Build a custom cancellable download engine: rejected because it duplicates the official updater for a behavior not required by the primary user flow.
- Exclude the whole native updater module from coverage: rejected because state and validation logic are pure and testable.

**Sources**: [Tauri API mocking](https://v2.tauri.app/develop/tests/mocking/), [Tauri WebDriver guidance](https://v2.tauri.app/develop/tests/webdriver/)

## Decision 9: Measure against the existing application baseline

**Decision**: Use the recorded 1,968,509-byte NSIS installer and existing startup/idle measurements as the pre-feature baseline, then record the same measurements with updater capability enabled.

**Rationale**: The constitution and specification require evidence for the 2 MiB footprint, startup, check latency, and idle-work budgets. Documentation cannot guarantee the compiled plugin's final size, so acceptance depends on measurement rather than estimation.

**Alternatives considered**:

- Assume the dependency fits: rejected because the footprint budget is a gate.
- Compare source dependency counts only: rejected because user-visible installed size and runtime cost are the actual constraints.
