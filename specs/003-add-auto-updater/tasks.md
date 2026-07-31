# Tasks: Automatic Application Updates

**Input**: Design documents from `/specs/003-add-auto-updater/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Tests are required by the feature specification and constitution. Write the named tests first, confirm they fail for the intended missing behavior, then implement. Frontend and Rust production code must each retain at least 80% line coverage.

**Organization**: Tasks are grouped by user story so detection, installation, and release publication can be implemented and validated as distinct increments.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add the one justified runtime dependency and capture immutable pre-feature measurements before behavior changes.

- [X] T001 [P] Add Windows-only `tauri-plugin-updater = "2"` and regenerate the locked Rust dependency graph in `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`
- [X] T002 [P] Record the pre-change first-render timing, idle CPU/memory, network activity, dependency list, and 1,968,509-byte installer baseline in `specs/003-add-auto-updater/validation/baseline.md`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish updater trust, endpoint configuration, and native plugin composition required by all user stories.

**⚠️ CRITICAL**: Complete this phase before user-story implementation. Never commit the updater private key or password.

- [X] T003 Generate the long-lived production updater keypair, provision `TAURI_SIGNING_PRIVATE_KEY` and optional `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` in the protected GitHub release environment, and record non-secret custody/recovery evidence in `specs/003-add-auto-updater/validation/signing-setup.md`
- [X] T004 Configure updater artifact generation, the committed public-key content, `https://github.com/msugenius/periscope/releases/latest/download/latest.json`, and passive Windows installation in `src-tauri/tauri.conf.json`
- [X] T005 [P] Register the native updater plugin in `src-tauri/src/lib.rs` and verify the custom-command design requires no updater JavaScript/process dependency or permission expansion in `package.json` and `src-tauri/capabilities/default.json`

**Checkpoint**: The application can initialize the trusted native updater, but no automatic check or installation behavior exists yet.

---

## Phase 3: User Story 1 - Learn that an update is available (Priority: P1) 🎯 MVP

**Goal**: Check once per application process after the UI is usable and present only a newer compatible stable release without disrupting normal operation.

**Independent Test**: Launch an older build against a controlled endpoint containing a newer stable signed release; verify Settings and the crosshair remain usable, exactly one native check occurs across settings-window recreation, and the offered version/summary appears once. Equal, lower, prerelease, malformed, incompatible, offline, timeout, and rate-limited cases must not produce a false offer.

### Tests for User Story 1

- [X] T006 [P] [US1] Write failing Rust tests for session phases, once-per-process checking, stable SemVer filtering, installed/equal/lower versions, malformed metadata, incompatible platform, offline/timeout/rate-limit failures, safe error mapping, and pending-resource cleanup in `src-tauri/src/updater.rs`
- [X] T007 [P] [US1] Write failing jsdom tests for offered/no-update/error rendering, escaped release notes, non-blocking first render, subscribe-before-snapshot ordering, and recreated-webview state recovery in `src/update-ui.test.ts` and `src/app.test.ts`

### Implementation for User Story 1

- [X] T008 [US1] Implement `InstalledVersion`, `ReleaseCandidate`, `UpdateSession`, the updater adapter boundary, once-only background check, `get_update_status`, `start_update_check`, and full-state event emission without holding locks across awaits in `src-tauri/src/updater.rs`
- [X] T009 [US1] Manage the process-scoped updater state and register the read/check commands while keeping `src-tauri/src/lib.rs` as a composition boundary in `src-tauri/src/lib.rs`
- [X] T010 [P] [US1] Implement the typed snapshot model plus safe available/no-update/check-failure rendering in `src/update-ui.ts`
- [X] T011 [US1] Add accessible update offer, status, and failure presentation without changing the framework-free layout in `src/styles.css`
- [X] T012 [US1] Compose the updater UI after `renderShell()`, subscribe before reading the snapshot, start the check outside the boot critical path, and clean up only webview listeners in `src/app.ts`
- [X] T013 [US1] Run the focused Rust/Vitest tests plus both coverage gates and record the independent US1 results and request-count evidence in `specs/003-add-auto-updater/validation/us1-detection.md`

**Checkpoint**: Update detection and notification work independently against a controlled metadata endpoint; no package is downloaded or installed.

---

## Phase 4: User Story 2 - Install an available update safely (Priority: P2)

**Goal**: Let the user approve or dismiss the exact offered Windows update, show progress, and preserve a runnable installation on failure.

**Independent Test**: Offer a valid signed newer build from a disposable static endpoint, approve it, and verify the pinned installer is verified, applied, and restarted as the approved version with settings preserved. Decline, version mismatch, corruption, permission/storage failure, duplicate action, and download/install failure must leave the prior installation runnable or unchanged.

### Tests for User Story 2

- [X] T014 [P] [US2] Write failing Rust tests for candidate/version pinning, dismissal, duplicate install suppression, monotonic progress, signature/download/install failures, safe retry eligibility, and release of the pending update object in `src-tauri/src/updater.rs`
- [X] T015 [P] [US2] Write failing jsdom tests for approve/dismiss controls, disabled duplicate actions, download/install progress, actionable failures, retry visibility, and Windows restart messaging in `src/update-ui.test.ts` and `src/app.test.ts`

### Implementation for User Story 2

- [X] T016 [US2] Implement `dismiss_update` and `install_update` using the exact retained updater object, validate the confirmed version, emit progress/failure snapshots, and delegate signature verification plus passive install/restart to Tauri in `src-tauri/src/updater.rs`
- [X] T017 [US2] Register the dismiss/install commands and coordinate updater process exit with existing tray/close lifecycle behavior in `src-tauri/src/lib.rs`
- [X] T018 [US2] Implement approve/dismiss bindings, accessible progress, duplicate-action guards, recovery text, and retry presentation in `src/update-ui.ts` and `src/styles.css`
- [ ] T019 [US2] Build two disposable signed versions and validate approve, decline, corruption, restart, installed version, settings preservation, two-instance concurrent install attempts, permission/storage failure, and prior-version recovery in an isolated Windows environment; record evidence in `specs/003-add-auto-updater/validation/us2-installation.md`
- [X] T020 [US2] Run the focused Rust/Vitest tests plus both coverage gates and record independent US2 results in `specs/003-add-auto-updater/validation/us2-automated.md`

**Checkpoint**: A controlled signed update can be installed safely with explicit user approval; production GitHub publication is not yet required.

---

## Phase 5: User Story 3 - Publish updater-ready semantic releases (Priority: P3)

**Goal**: Publish one verified `vMAJOR.MINOR.PATCH` Windows release from each valid release-ready `master` merge, with signed updater assets and deterministic metadata.

**Independent Test**: Merge a valid version-increasing change in a disposable repository configured like production and verify the exact merge SHA produces one stable release with matching installer, signature, `latest.json`, manifest, notes, tag, and packaged version. Invalid, lower, reused, partial, conflicting, failed, and retried paths must not expose a bad update.

### Tests for User Story 3

- [X] T021 [P] [US3] Write failing PowerShell tests for strict numeric SemVer, leading-zero/prerelease/build rejection, five-location version agreement, greatest-stable selection, legacy/draft/prerelease filtering, migration floor, retry/conflict state, deterministic handoff, updater metadata, and public manifest validation in `scripts/release/Test-ReleasePipeline.ps1`

### Implementation for User Story 3

- [X] T022 [US3] Implement reusable pure version parsing/comparison, declaration loading, release filtering, file-evidence, handoff, metadata, and public-manifest helpers in `scripts/release/ReleasePipeline.psm1`
- [X] T023 [US3] Set the release-ready application version to `0.2.0` when the stable floor is still `0.1.0`—otherwise choose the next valid minor version—and synchronize `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `package.json`, and `package-lock.json`
- [X] T024 [US3] Run release-helper tests and strict source/lock version-agreement validation without secrets after dependency installation in `.github/workflows/quality.yml`
- [X] T025 [US3] Replace SHA release identity with early verified `v<version>` retry/conflict inspection, greatest-stable comparison, exact-merge build, release-only signing secrets, signed NSIS output, and deterministic private handoff in `.github/workflows/release.yml`
- [X] T026 [US3] Implement publisher-side handoff validation, SemVer-ordered generated notes, immutable tagged URLs, literal signature metadata, deterministic public manifest, four-asset draft verification, publish-last/latest mutation, and idempotent resume/no-op behavior in `.github/workflows/release.yml`
- [X] T027 [P] [US3] Document stable-channel behavior, source-declared version bumps, signing-key custody/recovery, required GitHub secrets/environment, four release assets, and the first SemVer migration in `README.md`
- [ ] T028 [US3] Exercise valid first/later releases, invalid/non-increasing versions, missing signing secrets, conflicting tag/SHA/assets, resumable drafts, completed reruns, and two queued increasing versions in a disposable GitHub repository; record evidence in `specs/003-add-auto-updater/validation/us3-release.md`
- [ ] T029 [US3] Compare the published tag, packaged version, notes, `latest.json`, installer/signature bytes, public manifest, source SHA, publication time, and latest designation against `specs/003-add-auto-updater/contracts/release-artifacts.md`; time the source-to-artifact traceability check against the two-minute target and record the result in `specs/003-add-auto-updater/validation/us3-contract.md`

**Checkpoint**: Production CD can supply the stable signed metadata and artifacts consumed by US1 and US2.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Complete documentation, full quality evidence, performance/footprint measurement, and final constitutional review.

- [X] T030 [P] Reconcile the final command names, test endpoint procedure, and evidence links with the implemented behavior in `specs/003-add-auto-updater/quickstart.md`
- [X] T031 Run `npm run format:check`, `npm run lint`, `npm run test:coverage`, and `scripts/release/Test-ReleasePipeline.ps1`; confirm at least 80% line coverage for both instrumented codebases and record results/exclusion rationale in `specs/003-add-auto-updater/validation/quality.md`
- [ ] T032 Measure p95 first-render delta, normal check latency, automatic request count, post-check idle CPU/memory, temporary-data cleanup, and installed-size delta against the baseline; record budget evidence in `specs/003-add-auto-updater/validation/performance-footprint.md`
- [X] T033 Review updater/release changes for KISS, DRY, module ownership, unused dependencies, polling, lock scope, pending-resource cleanup, escaped external text, least privilege, secret leakage, immutable URLs/assets, and recovery behavior; record findings in `specs/003-add-auto-updater/validation/review.md`
- [ ] T034 Execute every applicable scenario in `specs/003-add-auto-updater/quickstart.md` and link the resulting automated, controlled Windows, GitHub, performance, and security evidence from `specs/003-add-auto-updater/validation/quickstart-results.md`
- [ ] T035 [P] Run a first-attempt usability check with representative users and verify at least 90% can identify the offered version and start or dismiss the update without external instructions; record method and results in `specs/003-add-auto-updater/validation/usability.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; T001 and T002 can run in parallel.
- **Foundational (Phase 2)**: Depends on Phase 1. T004 depends on the public key from T003; T005 can run in parallel with T003/T004 after T001.
- **User Story 1 (Phase 3)**: Depends on Phase 2 and supplies the process-scoped candidate/check contract.
- **User Story 2 (Phase 4)**: Depends on US1 because installation consumes US1's retained candidate and state/event contract.
- **User Story 3 (Phase 5)**: Depends only on Phase 2 and can run in parallel with US1/US2; production rollout of US1/US2 depends on its published endpoint.
- **Polish (Phase 6)**: Depends on every user story selected for delivery; full release readiness requires all three.

### User Story Dependency Graph

```text
Setup → Foundation → US1 Detection → US2 Safe Installation
                   └──────────────→ US3 SemVer Publication

Production rollout: US1 + US2 + US3
```

### Within Each User Story

- Write the named tests first and confirm they fail for the intended missing behavior.
- Define/verify models and contracts before orchestration.
- Implement native/service behavior before final composition.
- Complete the independent test and coverage record before declaring the story done.
- Never make a failed test pass by excluding maintained updater logic from coverage.

### Parallel Opportunities

- Setup dependency work (T001) and baseline capture (T002) are independent.
- Native and frontend failing tests for US1 (T006, T007) can run in parallel.
- After tests exist, native checking (T008/T009) and the frontend view model (T010) can progress in parallel against `contracts/updater-ipc.md`.
- Native and frontend failing tests for US2 (T014, T015) can run in parallel.
- US3 release-helper tests/documentation (T021, T027) can progress separately from native updater implementation after Foundation.
- US3 as a whole can run alongside US1/US2 after Phase 2, with final controlled end-to-end validation waiting for all paths.

---

## Parallel Example: User Story 1

```text
Task T006: Write Rust updater state/check tests in src-tauri/src/updater.rs
Task T007: Write jsdom detection/presentation tests in src/update-ui.test.ts and src/app.test.ts

After both test contracts are fixed:
Task T008/T009: Implement native checking and command registration
Task T010: Implement frontend snapshot rendering
```

## Parallel Example: User Story 2

```text
Task T014: Write Rust pin/dismiss/install/progress/failure tests
Task T015: Write frontend approve/dismiss/progress/recovery tests
```

## Parallel Example: User Story 3

```text
Task T021/T022: Build and test the pure release helper
Task T027: Document the already-approved release/signing contracts in README.md
```

---

## Implementation Strategy

### MVP First: User Story 1

1. Complete Setup and Foundational phases.
2. Complete US1 tests and implementation.
3. Validate detection independently against a controlled signed metadata endpoint.
4. Stop and review startup/request/idle evidence.

This MVP demonstrates correct update discovery and user notification. It is not production-deployable until US3 publishes the trusted stable endpoint.

### Incremental Delivery

1. **Foundation**: Trusted plugin configuration and baseline evidence.
2. **US1**: One-check-per-process detection and safe notification.
3. **US2**: User-approved pinned download/install with progress and recovery.
4. **US3**: Monotonic signed SemVer releases and immutable metadata.
5. **Polish**: Full coverage, controlled replacement, release, performance, footprint, security, and quickstart evidence.

### Parallel Team Strategy

After Foundation:

- Developer A: US1 native detection and frontend notification.
- Developer B: US3 release helper and CD publication.
- Developer C: Prepare US2 tests/UI against the fixed IPC contract, then complete native installation after US1 candidate ownership lands.

Merge in dependency order: US1 before US2; US3 may merge independently, but enable production auto-update only when all three contracts agree.

## Notes

- `[P]` tasks touch different files or external setup and can proceed without waiting for another incomplete task in the same dependency tier.
- `[US1]`, `[US2]`, and `[US3]` map directly to the prioritized user stories in `spec.md`.
- Never commit updater private-key material or place it in workflow artifacts, caches, test fixtures, logs, or validation documents.
- Use disposable keys, repositories, endpoints, and Windows installations for destructive updater/release tests.
- Declining before download is the supported cancellation path; custom mid-download cancellation is outside scope.
- Do not add prerelease channels, schedulers, polling, JavaScript updater/process plugins, custom installers, or a separate update service.
