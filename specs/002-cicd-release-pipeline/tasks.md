---

description: "Dependency-ordered implementation tasks for CI/CD and Windows releases"
---

# Tasks: CI/CD and Windows Releases

**Input**: Design documents from `/specs/002-cicd-release-pipeline/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Tests are required by the specification and constitution. Test and controlled-integration tasks below cover critical behavior, error paths, workflow boundaries, and the independent 80% line-coverage floor for TypeScript and Rust.

**Organization**: Tasks are grouped by user story so branch validation, PR gating, and release publication can be implemented and verified as independently as their shared workflow surfaces allow.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it uses different files and has no dependency on another incomplete task
- **[Story]**: Maps the task to a user story from spec.md
- Every task includes the exact file path or evidence artifact it changes

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add only the development-time tools and pinned toolchain needed by the planned quality and release workflows.

- [X] T001 Add exact development dependencies for Prettier, Vitest, V8 coverage, and jsdom and update the lockfile in package.json and package-lock.json
- [X] T002 [P] Pin Rust 1.97.0 with rustfmt, clippy, and llvm-tools-preview components in rust-toolchain.toml
- [X] T003 [P] Exclude dependencies, generated schemas, caches, build output, coverage output, and Tauri target output from formatting in .prettierignore

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish one authoritative local/CI quality command surface and its coverage policy before implementing any workflow.

**CRITICAL**: Complete this phase before user-story work.

- [X] T004 Define format, format:check, lint, test, test:coverage:frontend, test:coverage:rust, and test:coverage orchestration with locked Cargo flags in package.json
- [X] T005 [P] Configure jsdom tests, maintained-TypeScript inclusion, declaration/generated exclusions, and an independent 80% frontend line threshold in vitest.config.ts
- [X] T006 Capture the pre-feature build, formatting, linting, tests, missing-coverage state, application dependency list, and installer-size baseline in specs/002-cicd-release-pipeline/validation/baseline.md

**Checkpoint**: Contributors and workflows can invoke the same ordered commands; story implementation may begin.

---

## Phase 3: User Story 1 - Validate every branch change (Priority: P1) MVP

**Goal**: Every branch push reports format then lint, stops after a formatting failure, and excludes tag-only pushes.

**Independent Test**: Push a clean change and a known formatting failure to disposable branches, including one with a slash in its name; verify the clean run performs format then lint and the failing run skips lint.

### Tests for User Story 1

- [X] T007 [US1] Write the fail-first branch-push event, ordering, failure, slash-name, tag-exclusion, and 5-minute acceptance matrix in specs/002-cicd-release-pipeline/validation/us1-branch-quality.md

### Implementation for User Story 1

- [X] T008 [US1] Create the pinned Windows quality job with read-only permissions, locked installs, safe npm/Cargo caches, per-ref cancellation, a 10-minute timeout, and push-stage order format then lint in .github/workflows/quality.yml
- [ ] T009 [US1] Execute the clean, formatting-failure, lint-failure, slash-branch, and tag-exclusion scenarios and record run links, stage states, diagnostics, and durations in specs/002-cicd-release-pipeline/validation/us1-branch-quality.md

**Checkpoint**: User Story 1 is a deployable MVP and is independently demonstrated on ordinary branches.

---

## Phase 4: User Story 2 - Gate pull requests to dev and master (Priority: P2)

**Goal**: PRs targeting `dev` or `master` report format, lint, and tests in order, enforce at least 80% line coverage separately for TypeScript and Rust, and expose one stable required check.

**Independent Test**: Open and update passing and intentionally failing PRs against both protected branches; verify ordered fail-fast checks, independent coverage failures, cancellation of superseded revisions, and branch-protection enforcement.

### Tests for User Story 2

> Write these tests before the production refactors and confirm they fail for the missing modules or uncovered behavior.

- [X] T010 [P] [US2] Add failing unit tests for HTML escaping, hotkey display, modifier detection, shortcut normalization, and keyboard-event conversion in src/ui-model.test.ts
- [X] T011 [P] [US2] Add failing jsdom tests for settings rendering, recording suppression, native invocation errors, save failures, and recovery behavior in src/app.test.ts
- [X] T012 [P] [US2] Add Rust tests for shortcut parsing, duplicate/conflict errors, recording suppression, press/release deduplication, and rollback-safe state in src-tauri/src/hotkeys.rs
- [X] T013 [P] [US2] Add Rust rasterization tests for geometry boundaries, outline/fill colors, transparency, and malformed-color fallback in src-tauri/src/overlay.rs
- [X] T014 [P] [US2] Add Rust tests for settings load defaults, malformed-file recovery, atomic persistence errors, and serialized round trips in src-tauri/src/lib.rs
- [X] T015 [US2] Run the new frontend and Rust tests before implementation and record the expected failures and uncovered production paths in specs/002-cicd-release-pipeline/validation/us2-pr-quality.md

### Implementation for User Story 2

- [X] T016 [US2] Extract and export pure escaping and hotkey conversion behavior from src/main.ts into src/ui-model.ts without changing user-visible behavior
- [X] T017 [US2] Move testable DOM/settings orchestration into src/app.ts, keep src/main.ts as the minimal boot boundary, and satisfy the jsdom behavior and error-path tests
- [X] T018 [US2] Isolate only the necessary pure hotkey, rasterization, and persistence logic from platform boundaries and satisfy Rust tests in src-tauri/src/hotkeys.rs, src-tauri/src/overlay.rs, and src-tauri/src/lib.rs
- [X] T019 [US2] Extend .github/workflows/quality.yml with opened, reopened, synchronize, and edited PR events for dev/master, conditional coverage-tool installation and test execution, one stable Quality / quality check, and PR-number cancellation
- [X] T020 [US2] Close remaining meaningful coverage gaps with focused tests in src/*.test.ts and src-tauri/src/*.rs, keep exclusions narrow in vitest.config.ts and package.json, and record separate passing coverage reports in specs/002-cicd-release-pipeline/validation/us2-pr-quality.md
- [ ] T021 [US2] Apply the dev/master required-PR and Quality / quality rules from contracts/repository-settings.md and record passing, stage-failure, coverage-failure, retargeting, superseded-run, and blocked-merge evidence in specs/002-cicd-release-pipeline/validation/repository-settings.md

**Checkpoint**: User Story 2 independently proves that eligible PRs cannot satisfy the required check without ordered validation and 80% coverage in both codebases.

---

## Phase 5: User Story 3 - Publish a Windows release from master (Priority: P3)

**Goal**: A merged PR into `master` builds its exact commit as one x64 NSIS installer and publishes exactly one traceable release with verified assets and generated notes.

**Independent Test**: Merge a passing PR to `master`; verify exact-SHA checkout, manifest/installer digest agreement, deterministic tag, generated changelog, final publication, safe retry, and no release for unmerged/direct-push paths.

### Tests for User Story 3

- [X] T022 [US3] Write the fail-first merged/unmerged/direct-push trigger, exact-SHA, manifest, build-failure, first/later changelog, retry, conflict, rapid-merge, permission, and 20-minute acceptance matrix in specs/002-cicd-release-pipeline/validation/us3-windows-release.md

### Implementation for User Story 3

- [X] T023 [US3] Create the merged-PR-to-master trigger, merged condition, master-release queue:max concurrency, exact merge-SHA checkout, pinned toolchain/actions, read-only build job, and x64 NSIS-only Tauri build in .github/workflows/release.yml
- [X] T024 [US3] Generate and validate the schema-v1 SHA-256 release manifest, require exactly one non-empty NSIS installer, and upload the short-retention windows-merge-SHA handoff artifact in .github/workflows/release.yml
- [X] T025 [US3] Add the dependent no-checkout publisher with contents:write as its only elevated permission, exact artifact download, trusted-context manifest validation, and deterministic release-merge-SHA identity in .github/workflows/release.yml
- [X] T026 [US3] Implement draft-first generated notes, explicit previous published tag selection, asset/digest verification, publish-last behavior, matching-release no-op retries, draft resume, and hard failure on identity conflicts in .github/workflows/release.yml
- [ ] T027 [US3] Execute unmerged-close, direct-push, cancelled/failed-build, first-release, and later-release scenarios and record event SHA, tag target, asset hashes, changelog range, permission boundaries, and durations in specs/002-cicd-release-pipeline/validation/us3-windows-release.md
- [ ] T028 [US3] Execute same-SHA rerun, interrupted-draft resume, conflicting-tag, and two-rapid-merge scenarios and record uniqueness, ordering, no-overwrite, and artifact/changelog association evidence in specs/002-cicd-release-pipeline/validation/us3-windows-release.md
- [ ] T029 [US3] Apply read-only default token and immutable-release settings where available from contracts/repository-settings.md and record the resulting repository release controls and unsigned-installer limitation in specs/002-cicd-release-pipeline/validation/repository-settings.md

**Checkpoint**: User Story 3 independently publishes a correct Windows release only for a successfully merged `master` PR.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Finish maintainability, documentation, security, performance, and footprint verification across all stories.

- [X] T030 [P] Configure reviewed npm and full-SHA GitHub Action update proposals without automatic release publication in .github/dependabot.yml
- [X] T031 [P] Document local quality commands, required checks, release identity, NSIS download, retry behavior, and unsigned SmartScreen warning in README.md
- [X] T032 Review changed source and workflows for KISS, DRY, explicit boundaries, removable duplication, and unused development dependencies, then record the runtime dependency diff and installer-size comparison in specs/002-cicd-release-pipeline/validation/footprint.md
- [ ] T033 Measure representative cold/warm branch, PR, and release runs against the 5/10/20-minute budgets and document cache state, sample size, results, and any justified remediation in specs/002-cicd-release-pipeline/validation/performance.md
- [ ] T034 Run every scenario in specs/002-cicd-release-pipeline/quickstart.md, verify both codebases remain at or above 80% line coverage and no secret appears in logs/artifacts, and record final results in specs/002-cicd-release-pipeline/validation/final.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies. T002 and T003 can run in parallel after work begins.
- **Foundational (Phase 2)**: Depends on Phase 1 and blocks workflow implementation.
- **User Story 1 (Phase 3)**: Depends on Phase 2.
- **User Story 2 (Phase 4)**: Test/refactor tasks T010-T018 depend on Phase 2 and may proceed alongside User Story 1; workflow task T019 depends on T008 because both update `.github/workflows/quality.yml`.
- **User Story 3 (Phase 5)**: Depends on Phase 2 and is technically independent of the quality workflow; public rollout tasks T027-T029 must wait for T021 so releases are sourced from protected, verified merges.
- **Polish (Phase 6)**: Depends on all three implemented stories; T030 and T031 can run in parallel.

### User Story Dependency Graph

```text
Setup → Foundation ─┬→ US1 branch validation ─────┐
                    ├→ US2 tests/refactors ───────┼→ Polish/final validation
                    └→ US3 release implementation ┘

US1 workflow ─────────────→ US2 PR workflow extension
US2 branch protection ────→ US3 public release validation
```

### Within User Story 1

1. T007 defines the fail-first acceptance matrix.
2. T008 implements branch validation.
3. T009 executes and records independent evidence.

### Within User Story 2

1. T010-T014 add tests in parallel.
2. T015 records their fail-first result.
3. T016-T018 implement the smallest testable boundaries.
4. T019 adds the PR gate after the shared US1 workflow exists.
5. T020 proves independent 80% coverage; T021 applies and verifies repository gates.

### Within User Story 3

1. T022 defines the fail-first release matrix.
2. T023-T026 build the release workflow sequentially because they modify one workflow and each stage depends on the prior contract.
3. T027 verifies first/later and non-release paths.
4. T028 verifies retry and concurrency paths.
5. T029 verifies repository-level release controls.

### Parallel Opportunities

- T002 and T003 use independent setup files.
- T005 can be prepared independently after the test dependencies in T001 are selected.
- T010-T014 are independent frontend/Rust test files or modules.
- US3 tasks T022-T026 can proceed while US1/US2 code tests and refactors are underway, but publication validation waits for branch protection.
- T030 and T031 use separate cross-cutting files.

---

## Parallel Example: User Story 1

User Story 1 intentionally stays sequential because its acceptance definition, single workflow implementation, and hosted verification all touch the same behavior:

```text
T007 → T008 → T009
```

## Parallel Example: User Story 2

After Phase 2, launch the fail-first tests together:

```text
Task T010: Add frontend pure-helper tests in src/ui-model.test.ts
Task T011: Add frontend DOM/Tauri tests in src/app.test.ts
Task T012: Add Rust hotkey tests in src-tauri/src/hotkeys.rs
Task T013: Add Rust rasterization tests in src-tauri/src/overlay.rs
Task T014: Add Rust persistence tests in src-tauri/src/lib.rs
```

Then complete T015 before the implementation tasks.

## Parallel Example: User Story 3

Release workflow mutation is sequential to keep permissions, identity, artifact, and publication state auditable:

```text
T022 → T023 → T024 → T025 → T026
```

Hosted US3 validation can run while US2's local coverage work finishes, but T027-T029 wait for protected-branch setup in T021.

---

## Implementation Strategy

### MVP First: User Story 1

1. Complete Setup and Foundational phases.
2. Complete T007-T009.
3. Stop and verify branch pushes independently.
4. Deliver the fast format/lint feedback loop before expanding the PR gate.

### Incremental Delivery

1. **MVP**: Any branch push receives ordered format/lint feedback.
2. **Protected integration**: Add tests, independent 80% coverage, PR event handling, and branch rules for `dev`/`master`.
3. **Automated distribution**: Add exact-SHA Windows packaging and safe release publication.
4. **Hardening**: Complete documentation, update automation, performance/footprint evidence, and the full quickstart.

### Parallel Team Strategy

After Phase 2:

- Developer A implements US1 and later extends the quality workflow in T019.
- Developer B writes US2 frontend/Rust tests and performs the minimal testability refactors.
- Developer C implements US3 through T026 without publishing externally.
- The team applies repository settings and performs hosted integration tests together in T021 and T027-T029.

---

## Notes

- Tests and acceptance matrices precede implementation and must demonstrate the missing/failing behavior first.
- `[P]` means the listed files do not conflict and prerequisites are already complete.
- Pin external actions to reviewed full commit SHAs with version comments.
- Do not use `pull_request_target`, personal tokens, broad write permissions, mutable published assets, or caches containing credentials.
- Do not exclude whole maintained production modules to reach coverage; extract pure logic or add focused tests first.
- Do not add runtime dependencies, multi-platform bundles, MSI packaging, code signing, or a release service outside the specified scope.
- Each validation task records evidence in `specs/002-cicd-release-pipeline/validation/` so implementation decisions remain auditable.
