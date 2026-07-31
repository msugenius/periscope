# Implementation Plan: Automatic Application Updates

**Branch**: `[003-add-auto-updater]` | **Date**: 2026-07-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-add-auto-updater/spec.md`

**Note**: This plan is produced by `/speckit-plan`. Task decomposition belongs to `/speckit-tasks`.

## Summary

Add a stable Windows update channel backed by the project's GitHub Releases. A dedicated native updater module uses Tauri's signed updater, checks `releases/latest/download/latest.json` once after the settings UI becomes usable, retains candidate and progress state for the lifetime of the process, and exposes a small command/event contract to a framework-free update UI. Release CD changes from SHA-named publications to monotonically increasing `vMAJOR.MINOR.PATCH` releases, signs the NSIS updater artifact, generates deterministic update metadata, and preserves the existing exact-merge, least-privilege, draft-first, verify-before-publish guarantees.

## Technical Context

**Language/Version**: Rust 1.97.0, edition 2024; TypeScript 7.0.2 on Node.js 24; PowerShell 7 and GitHub Actions YAML for release automation

**Primary Dependencies**: Tauri 2.11.x; Windows-only `tauri-plugin-updater` 2.x; existing Tauri API, Vitest/jsdom, GitHub Actions, GitHub Releases, GitHub CLI/REST, and `tauri-apps/tauri-action` v1. No updater JavaScript or process plugin is added.

**Storage**: No persistent application data change. Native memory holds one process-scoped update state and at most one pinned candidate. GitHub Releases stores the NSIS installer, updater signature, `latest.json`, and traceability manifest; the release workflow uses a short-lived private handoff artifact.

**Testing**: Rust unit tests for updater state, stable-version filtering, pinning, and failures; Vitest/jsdom tests for update presentation and actions; dependency-injected or mocked updater boundaries; PowerShell release-helper tests; controlled two-version Windows installation tests in a disposable environment

**Coverage Tooling**: `vitest run --coverage` with an 80% frontend line threshold; `cargo llvm-cov --locked --workspace --all-targets --all-features --fail-under-lines 80` for Rust; updater platform calls remain a narrow boundary while updater business/state logic is instrumented

**Target Platform**: Windows x64 desktop application distributed as an NSIS installer; stable GitHub Releases channel only

**Project Type**: Framework-free Tauri desktop application plus repository-hosted CI/CD automation

**Performance Goals**: No more than 100 ms p95 added interactive startup time; 95% of checks resolve within 10 seconds under normal service conditions; no periodic polling or post-check idle CPU regression; no more than 2 MiB added installed footprint

**Constraints**: Check only after first UI render and at most once per process; settings webviews are destroyed and recreated; user approval precedes download/install; no downgrade or prerelease channel; mandatory updater signatures; exact candidate pinning; failed update leaves current install runnable; 80% coverage per instrumented codebase; release secrets restricted to trusted merged-`master` CD

**Scale/Scope**: One Windows architecture, one stable channel, one GitHub-hosted metadata document, one candidate/update attempt per process, and one signed release for each valid release-ready merge into the primary release branch

## Constitution Check

*GATE: Passed before Phase 0 research and re-checked after Phase 1 design.*

- **Dead simple — PASS**: The official updater handles transport, SemVer comparison, signature verification, and Windows installation. One native module is required because the existing settings webview is intentionally destroyed and recreated. The first release supports one platform and one stable channel only.
- **Performance first — PASS**: The check starts after the first render without being awaited. The design carries the 100 ms startup, 10-second check, zero-polling, idle CPU, and footprint budgets into measurement scenarios, using the recorded 1,968,509-byte installer as the pre-feature footprint baseline.
- **Lightweight — PASS**: Only the Windows Rust updater plugin is added. Custom commands avoid JavaScript updater/process dependencies and capability expansion. One check occurs per process; pending resources are dropped on dismissal or terminal failure.
- **Modular — PASS**: `updater.rs` owns native lifecycle and update state, `update-ui.ts` owns presentation/actions, `lib.rs` and `app.ts` remain composition boundaries, and release helpers own pure SemVer/manifest rules outside workflow YAML.
- **KISS and DRY — PASS**: The configured application version remains the release authority; ecosystem declarations and locks are validated mirrors. The same SemVer parser and release-state helpers are used by local tests and CD. No generalized channel, scheduler, or release service is introduced.
- **Test quality — PASS**: Pure state and version rules receive automated boundary/error coverage. DOM behavior is covered with jsdom, Rust logic remains included in coverage, and installer replacement/elevation/restart are verified in controlled Windows scenarios where unit tests cannot provide evidence.

### Post-Design Re-check

Phase 1 introduces no constitution violation. Native state is necessary rather than speculative: hiding Settings destroys the webview, so frontend state cannot enforce once-per-process checking or retain an approved candidate. The private handoff and public manifest are separate because the publisher must validate untrusted build output and create `latest.json` only after release notes and immutable URLs are known. No background poller, persistent updater database, abstraction framework, extra web service, or prerelease channel is added.

## Project Structure

### Documentation (this feature)

```text
specs/003-add-auto-updater/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── release-artifacts.md
│   ├── updater-ipc.md
│   └── updater-metadata.md
├── checklists/
│   └── requirements.md
└── tasks.md                    # Created later by /speckit-tasks
```

### Source Code (repository root)

```text
.github/
└── workflows/
    ├── quality.yml             # Version-contract tests without release secrets
    └── release.yml             # SemVer validation, signed build, verified publication

scripts/
└── release/
    ├── ReleasePipeline.psm1    # Pure version, release-list, and manifest rules
    └── Test-ReleasePipeline.ps1

src/
├── app.ts                      # Compose update UI after first render
├── app.test.ts
├── update-ui.ts                # Update view model, rendering, and action binding
├── update-ui.test.ts
└── styles.css

src-tauri/
├── src/
│   ├── lib.rs                  # Register plugin/state/commands only
│   └── updater.rs              # Process-scoped updater orchestration and tests
├── Cargo.toml
├── Cargo.lock
└── tauri.conf.json             # Artifact generation, public key, endpoint, install mode

package.json                    # Mirrored release version and existing scripts
package-lock.json
README.md                       # Stable channel, update behavior, release/version procedure
```

**Structure Decision**: Retain the existing single Tauri project. The updater crosses native network/install and disposable webview boundaries, so it receives one cohesive native module and one cohesive UI module. Release rules move into a small PowerShell module because extending the existing 400-line inline workflow would obscure SemVer and idempotency behavior; the workflow remains orchestration rather than a second implementation of those rules.

## Implementation Strategy

### Native updater lifecycle

- Add `tauri-plugin-updater` only for Windows, register it in `lib.rs`, and configure the committed public key, stable endpoint, passive Windows installation, and updater artifact creation.
- Create `updater.rs` with process-scoped state and a narrow adapter around the plugin. The mutex protects only short state reads/transitions and is never held across network, download, or install awaits.
- The first rendered webview subscribes to updater state, loads the current snapshot, and invokes `start_update_check` without awaiting it. Native state makes this command idempotent when a later settings webview is created.
- Keep the exact plugin update object for an offered candidate. Installation accepts the displayed version, rejects mismatches, and consumes that pinned object so a changing “latest” release cannot redirect an approved attempt.
- Dismissal drops the pending object for the process session. Check, metadata, signature, download, and install failures transition to a user-safe status without interrupting overlay, tray, hotkey, settings, or shutdown behavior.
- Use the official Windows updater restart path. Do not add a process plugin or custom self-replacement logic.

### Framework-free update UI

- Add a modal/banner surface that shows installed version, offered version, release notes, install/dismiss actions, progress, and actionable terminal failures.
- Keep rendering and binding in `update-ui.ts`; `app.ts` supplies the container and composes it with the existing shell. Escape all release-provided text before insertion.
- Subscribe before reading the snapshot so a recreated webview cannot miss a transition. Re-render from native state rather than preserving a second authoritative frontend state machine.
- Disable duplicate actions while an operation is active. Dismissal suppresses prompts until process exit; reopening Settings displays any still-active offer or progress state.

### Semantic version authority

- Continue using the version in `src-tauri/tauri.conf.json` as the authoritative application/release version for the smallest migration from current CD.
- Require exact agreement with `src-tauri/Cargo.toml`, the periScope entry in `src-tauri/Cargo.lock`, `package.json`, and the root entry in `package-lock.json`. Quality runs validate strict `MAJOR.MINOR.PATCH` syntax and agreement; release CD repeats validation against the exact merged source.
- Reject leading zeroes, prerelease/build suffixes, equal/lower versions, and any version already associated with another commit.
- The implementation release should move from the current `0.1.0` floor to `0.2.0` as a backward-compatible feature release, unless another stable SemVer release changes the floor before merge.

### Signed release and metadata publication

- Preserve the merged-PR-to-`master` trigger, exact merge-SHA checkout, serialized non-cancelled execution, build job with `contents: read`, and publisher-only `contents: write`.
- Before building, derive `v<version>`, inspect existing releases, and short-circuit a retry only when the public release targets the same SHA and all four public assets match its manifest. Conflicting version/tag/SHA state fails before build.
- Compare numeric SemVer against the greatest published stable `vX.Y.Z` tag. Ignore drafts, prereleases, malformed tags, and historical `release-<sha>` tags for update ordering. The first SemVer release uses `0.1.0` as the migration floor and may use the latest legacy release only as its changelog predecessor.
- Supply `TAURI_SIGNING_PRIVATE_KEY` and its optional password only to the trusted signing step. Commit only the public key. Losing or rotating the private key requires an explicit migration because installed clients trust its public counterpart.
- Build one x64 NSIS installer plus its mandatory updater signature. The private handoff includes those files and a deterministic handoff manifest without timestamps or workflow-run identifiers.
- The publisher validates the handoff, generates release notes, creates `latest.json` with an immutable version-tagged installer URL and literal signature, then creates a deterministic public `release-manifest.json` covering the installer, signature, metadata, version, and source SHA.
- Create or resume a matching draft, upload exactly the installer, signature, `latest.json`, and public manifest, verify names/sizes/digests and tag target, then publish and mark latest as the final mutation. A correct published retry is a no-op; a mismatch is never overwritten.

### Test and measurement strategy

- Rust tests cover state transitions, once-per-process behavior, stable candidate validation, installed/equal/lower/prerelease cases, pin mismatch, dismissal, duplicate actions, and error sanitization. Network/plugin/install calls are injected at the narrow boundary.
- Frontend tests cover offer text, safe notes rendering, approve/dismiss, progress, failures, recreated-webview snapshots, and non-blocking boot behavior.
- Release-helper tests cover numeric SemVer ordering, all declaration locations, legacy/draft/prerelease filtering, highest-version selection, migration floor, release conflicts, deterministic handoff, and metadata/artifact validation.
- Controlled Windows tests use disposable updater keys and two genuinely signed versions to verify signature rejection, passive installation, process exit/restart, final version, settings preservation, elevation denial, insufficient storage, and rollback safety.
- Record pre/post first-render timing, check latency, idle CPU, retained memory, network-request count, and installed size. Compare footprint against the recorded 1,968,509-byte NSIS baseline and fail acceptance if the installed delta exceeds 2 MiB.

## Complexity Tracking

No constitution violations require justification.
