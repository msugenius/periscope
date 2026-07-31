# Research: CI/CD and Windows Releases

## Decision 1: Use GitHub Actions and GitHub Releases

**Decision**: Implement repository automation in `.github/workflows` and publish through GitHub Releases.

**Rationale**: The configured remote is GitHub, the requested concepts map directly to GitHub branch/PR events and releases, and no external service or credential is needed. GitHub documents branch filters, PR base-branch filters, merged-PR conditions, permissions, artifacts, and generated release notes.

**Alternatives considered**:

- External CI/release service: rejected because it adds credentials, configuration, and another operational dependency.
- Local-only hooks: rejected because they cannot provide shared required checks or publish releases.

**Sources**: [Workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax), [pull request events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request), [merged pull request pattern](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#running-your-pull_request-workflow-when-a-pull-request-merges)

## Decision 2: Use one ordered quality job and one authoritative command surface

**Decision**: Define root package scripts for format, lint, and coverage tests; invoke them as sequential steps in one stable quality job. Push runs omit the final coverage step; eligible PR runs include it.

**Rationale**: A single job naturally stops after the first failure, gives branch protection one unambiguous check name, and avoids duplicated runner setup. The same scripts let contributors reproduce CI locally.

**Alternatives considered**:

- Separate format, lint, and test jobs linked with `needs`: rejected because they add runner/setup overhead and three status checks without adding parallelism.
- Separate push and PR workflows: rejected because setup and quality ordering would be duplicated.
- CI-only shell commands: rejected because local and hosted validation would drift.

**Sources**: [Required status checks](https://docs.github.com/en/repositories/configuring-branches-and-merges/managing-protected-branches/about-protected-branches#require-status-checks-before-merging), [troubleshooting required checks](https://docs.github.com/en/repositories/configuring-branches-and-merges/managing-rulesets/troubleshooting-rules#troubleshooting-required-status-checks)

## Decision 3: Prettier plus native ecosystem linting

**Decision**: Add Prettier as the single frontend formatter, use strict TypeScript compilation as frontend static linting, and use built-in `rustfmt` and Clippy for Rust.

**Rationale**: Prettier covers maintained TypeScript, HTML, CSS, JSON, Markdown, and workflow YAML. The existing strict TypeScript compiler already detects frontend type and correctness errors. Clippy is Rust's supported linter and can deny warnings in CI. These are development-only tools.

**Alternatives considered**:

- ESLint plus TypeScript ESLint plus Prettier: rejected for initial scope because it adds several packages and overlapping configuration without a demonstrated lint rule requirement beyond strict type checking.
- Biome alone: rejected because its current official TypeScript language support is 5.9 while this repository uses TypeScript 7.0.2.
- Formatting only changed files: rejected because the gate must establish repository-wide consistency.

**Sources**: [Prettier CI check](https://prettier.io/docs/api), [Clippy CI guidance](https://doc.rust-lang.org/clippy/continuous_integration/index.html), [Clippy usage](https://doc.rust-lang.org/stable/clippy/usage.html), [Biome language support](https://biomejs.dev/internals/language-support/)

## Decision 4: Gate TypeScript and Rust coverage independently

**Decision**: Use Vitest with V8 coverage and jsdom for TypeScript, and `cargo-llvm-cov` for Rust. Each instrumented production codebase must independently reach 80% line coverage.

**Rationale**: Vitest integrates with the existing Vite project and supports enforceable line thresholds. `cargo-llvm-cov` supports Windows MSVC and a fail-under-lines threshold without adding application dependencies. Separate gates prevent a well-tested codebase from masking another.

**Alternatives considered**:

- One combined coverage percentage: rejected because it violates the constitution's per-codebase floor.
- Browser end-to-end tests as the only frontend coverage: rejected as slower and less focused than unit/jsdom tests for this small framework-free UI.
- Tarpaulin for Rust: rejected because Windows MSVC is not its strongest supported path and `cargo-llvm-cov` directly uses Rust LLVM coverage.
- Broad platform-module exclusions: rejected; pure behavior must be extracted and tested before narrow boundary exclusions are accepted.

**Sources**: [Vitest coverage thresholds](https://v3.vitest.dev/config/), [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov)

## Decision 5: Pin Windows, Node, Rust, and action inputs

**Decision**: Use `windows-2025`, Node.js 24, Rust 1.97.0, locked npm/Cargo installs, and verified full commit SHA references for external actions.

**Rationale**: The source and installer are Windows-specific. Pinning removes unplanned toolchain drift and makes performance and lint results reproducible. GitHub identifies a full action commit SHA as the safest immutable reference.

**Alternatives considered**:

- `windows-latest`, Node `lts/*`, and Rust `stable`: rejected for the primary gate because each can change without a repository change.
- Linux quality runners: rejected because Win32 dependencies and platform behavior are part of the maintained Rust codebase.
- Caching `node_modules`: rejected; cache package downloads and locked installs instead.

**Sources**: [Tauri GitHub pipeline](https://v2.tauri.app/distribute/pipelines/github/), [GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners), [dependency caching](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching), [secure action use](https://docs.github.com/en/actions/reference/security/secure-use#using-third-party-actions)

## Decision 6: Build exactly the merged commit as one x64 NSIS installer

**Decision**: Trigger release processing only from a merged/closed PR to `master`, check out its exact merge commit, and build only the x64 NSIS installer.

**Rationale**: The event and condition exclude direct pushes and unmerged PRs. Explicit SHA checkout prevents a later `master` update from changing the build input. NSIS produces the requested end-user installer without MSI's additional WiX/VBScript needs; a plain portable executable is not officially supported by Tauri.

**Alternatives considered**:

- Trigger on pushes to `master`: rejected because it would also release direct pushes.
- Build all Tauri bundle targets: rejected because one Windows installer satisfies the requirement with less time and storage.
- Publish MSI and NSIS: rejected until a distinct MSI consumer requirement exists.
- Cross-compile from Linux: rejected because Tauri recommends Windows CI when available.

**Sources**: [Tauri Windows installers](https://v2.tauri.app/distribute/windows-installer/), [Tauri action](https://github.com/tauri-apps/tauri-action), [merged PR event](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#running-your-pull_request-workflow-when-a-pull-request-merges)

## Decision 7: Use the full merge SHA as the release identity

**Decision**: Tag every merge release as `release-<full-merge-sha>` and include the application version, PR number, and short SHA in the human title.

**Rationale**: Every `master` merge must release even when the application version is unchanged. The full SHA is deterministic, unique, collision-resistant, traceable, and naturally idempotent across retries.

**Alternatives considered**:

- `v<application-version>`: rejected because the current version is duplicated and is not guaranteed to change for every PR.
- Run number or timestamp tag: rejected because reruns would produce a different identity for the same merge.
- Mutating the application version during CI: rejected because it makes the built inputs differ from the merged source and complicates upgrade semantics.

**Sources**: [Tauri versioning](https://v2.tauri.app/reference/config/#version), [GitHub release CLI](https://cli.github.com/manual/gh_release_create)

## Decision 8: Split build from publication and publish draft last

**Decision**: Build with read-only permissions, transfer an installer plus SHA-256 manifest, and give `contents: write` only to a dependent publisher. The publisher creates or resumes a deterministic draft, generates notes, verifies identity/assets, and publishes last.

**Rationale**: Application build code never receives a write token. A build failure creates no release; an upload failure leaves at most a non-public draft. A published matching release can be verified and treated as an idempotent success.

**Alternatives considered**:

- Build and publish in one job: rejected because the write token would be available while application-controlled build commands execute.
- Publish first, upload later: rejected because users could see an incomplete release.
- Blind asset overwrite on retry: rejected because an interrupted replacement can destroy a valid published asset.

**Sources**: [Workflow permissions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#permissions), [passing artifacts between jobs](https://docs.github.com/en/actions/tutorials/store-and-share-data#passing-data-between-jobs), [immutable release guidance](https://docs.github.com/en/enterprise-cloud@latest/code-security/concepts/supply-chain-security/immutable-releases)

## Decision 9: Queue releases and generate notes from the prior publication

**Decision**: Use one release concurrency group with `queue: max` and no cancellation. Determine the previous published tag immediately before draft creation and request GitHub-generated notes with that explicit starting tag; omit it for the first release.

**Rationale**: GitHub supports up to 100 pending runs with `queue: max`. Serial publication prevents artifacts and changelog ranges from racing, while generated notes provide merged PRs, contributors, and a full comparison link.

**Alternatives considered**:

- Default concurrency: rejected because a newer pending run replaces an older one.
- `cancel-in-progress: true`: rejected because every successful merge must release.
- Concurrent publication: rejected because generated-note ranges could overlap or appear out of order.
- Handwritten changelog text: rejected because GitHub already supplies the requested traceable information.

**Sources**: [GitHub concurrency queue](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#concurrency), [generated release notes](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes), [generate-notes API](https://docs.github.com/en/rest/releases/releases#generate-release-notes-content-for-a-release)

## Decision 10: Leave Windows code signing outside this feature

**Decision**: Publish an unsigned NSIS installer and document that SmartScreen may warn. Add signing only when a certificate or managed signing service is explicitly provided.

**Rationale**: The request does not include certificate acquisition or secret provisioning. Tauri permits unsigned installers, while production signing requires an external credential and operational policy.

**Alternatives considered**:

- Generate or embed a certificate: rejected because self-issued signing does not establish publisher trust and private keys must not be invented or committed.
- Block all releases until signing exists: rejected because signing was not a stated acceptance condition.

**Source**: [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/)
