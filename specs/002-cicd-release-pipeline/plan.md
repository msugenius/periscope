# Implementation Plan: CI/CD and Windows Releases

**Branch**: `[002-cicd-release-pipeline]` | **Date**: 2026-07-31 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-cicd-release-pipeline/spec.md`

**Note**: This plan is produced by `/speckit-plan`. Task decomposition belongs to `/speckit-tasks`.

## Summary

Add GitHub Actions automation with one ordered quality workflow and one Windows release workflow. Root package scripts become the authoritative local and CI commands for format, lint, and coverage tests across the TypeScript and Rust codebases. Any branch push runs format then lint; pull requests targeting `dev` or `master` additionally run coverage-gated tests. A merged pull request to `master` is checked out by its exact merge SHA, built once as an x64 NSIS installer in a read-only Windows job, transferred with a SHA-256 manifest, and published by a separate least-privilege job as a deterministic, idempotent GitHub release with generated notes.

## Technical Context

**Language/Version**: GitHub Actions YAML; PowerShell 7 for Windows workflow steps; TypeScript 7.0.2 on Node.js 24; Rust 1.97.0, edition 2024

**Primary Dependencies**: GitHub Actions, Releases, and GitHub CLI/REST; Tauri 2.11.x; `tauri-apps/tauri-action` v1 in build-only mode; Prettier; Vitest with V8 coverage and jsdom; Rust `rustfmt`, Clippy, and `cargo-llvm-cov`

**Storage**: No application storage changes. GitHub stores workflow logs/statuses, dependency caches, short-lived build artifacts, Git tags, release metadata, generated notes, the NSIS installer, and its release manifest.

**Testing**: Vitest/jsdom for frontend behavior and pure modules; `cargo test` through `cargo-llvm-cov` for Rust logic; controlled GitHub integration scenarios for event filters, branch protection, artifact handoff, release retry, and publication

**Coverage Tooling**: `vitest run --coverage` with V8 global line threshold 80%; `cargo llvm-cov --locked --workspace --all-targets --all-features --fail-under-lines 80`; each report gates independently

**Target Platform**: GitHub-hosted Windows Server 2025 x64 runners; x64 Windows NSIS installer; installed application remains Windows-only

**Project Type**: Tauri desktop application with repository-level CI/CD automation

**Performance Goals**: 90% of branch quality runs within 5 minutes, PR quality runs within 10 minutes, and successful releases within 20 minutes of merge under normal GitHub availability

**Constraints**: Ordered fail-fast stages; minimum 80% line coverage per instrumented production codebase; exact merge-SHA builds; no release on direct `master` pushes or unmerged PRs; one release per merge SHA; generated changelog from the previous published release; no runtime dependency or runtime cost; publishing token exposed only to the publisher; external actions pinned to verified full commit SHAs

**Scale/Scope**: One small desktop app, two production codebases, all repository branches, PR gates for two protected branches, one release per merged `master` PR, and up to the hosting platform's 100-run serialized release queue

## Constitution Check

*GATE: Passed before Phase 0 research and re-checked after Phase 1 design.*

- **Dead simple — PASS**: Two workflows map directly to quality and release responsibilities. Root scripts provide one authoritative command set. The release build and publisher are separate only because FR-019 requires least-privilege publication.
- **Performance first — PASS**: The spec's 5/10/20-minute budgets are carried into workflow timeouts and validation. Locked installs and dependency caches address cold-start cost without changing the application.
- **Lightweight — PASS**: All added packages are development-only; the installed application gains no work, memory use, assets, or runtime dependencies. One x64 NSIS bundle is produced instead of every supported bundle type.
- **Modular — PASS**: Package scripts own quality commands, the quality workflow owns validation, the release build job owns compilation, and the publisher owns GitHub release state. Artifact and manifest contracts make boundaries explicit.
- **KISS and DRY — PASS**: Local and CI validation invoke the same scripts. Release identity is derived once from the full merge SHA. No reusable-workflow framework, release service, or speculative multi-platform matrix is introduced.
- **Test quality — PASS**: TypeScript and Rust coverage fail independently below 80%. Tests cover critical pure behavior and errors; controlled integration scenarios cover GitHub-only event and release boundaries. Exclusions are narrow, named, and justified.

### Post-Design Re-check

Phase 1 introduces no constitution violation. The manifest is a small handoff contract, not a new application model. Serial publication is required to preserve complete, ordered changelogs during rapid merges. Coverage-driven extraction is limited to separating testable logic from DOM, Tauri, and Win32 boundaries; pass-through layers are prohibited.

## Project Structure

### Documentation (this feature)

```text
specs/002-cicd-release-pipeline/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── quality-workflow.md
│   ├── release-manifest.md
│   ├── release-workflow.md
│   └── repository-settings.md
├── checklists/
│   └── requirements.md
└── tasks.md                 # Created later by /speckit-tasks
```

### Source Code (repository root)

```text
.github/
└── workflows/
    ├── quality.yml          # Push and protected-target PR validation
    └── release.yml          # Merged-master Windows build and publication

src/
├── main.ts                  # DOM/Tauri composition boundary
├── *.ts                     # Extracted testable frontend logic where coverage requires it
├── *.test.ts                # Frontend unit and jsdom behavior tests
├── styles.css
└── vite-env.d.ts

src-tauri/
├── src/
│   ├── lib.rs               # Tauri application composition boundary
│   ├── main.rs
│   ├── hotkeys.rs
│   ├── overlay.rs           # Pure logic retained/tested; Win32 boundary isolated if needed
│   └── settings.rs
├── Cargo.toml
├── Cargo.lock
└── tauri.conf.json

package.json                 # Authoritative format/lint/test scripts
package-lock.json
.prettierignore              # Ignore generated/build/vendor output
rust-toolchain.toml          # Rust 1.97.0 + rustfmt/clippy/llvm-tools-preview
vitest.config.ts             # jsdom setup, coverage scope, 80% line threshold
```

**Structure Decision**: Retain the existing Tauri single-project layout. CI/CD belongs under `.github/`; language-specific commands remain in their native package tools and are orchestrated by root package scripts. Production source is refactored only where a platform or UI boundary currently prevents meaningful coverage of otherwise pure behavior.

## Implementation Strategy

### Quality command surface

- Add `format`, `format:check`, `lint`, `test`, and `test:coverage` root scripts.
- `format:check` runs Prettier in check mode over maintained frontend, configuration, documentation, and workflow files, then `cargo fmt --check`.
- `lint` runs strict TypeScript checking, then Clippy for all Rust targets/features with warnings denied.
- `test:coverage` runs frontend and Rust coverage sequentially and preserves separate 80% gates and reports.
- Ignore generated schemas, dependencies, caches, compiled output, coverage output, and Tauri target output. Do not exclude maintained production modules merely to reach the threshold.

### Quality workflow

- Trigger branch validation with `push.branches: ['**']` so tag creation does not create redundant quality runs.
- Trigger PR validation for `opened`, `reopened`, `synchronize`, and `edited` events targeting `dev` or `master`.
- Use one stable `Quality / quality` job with ordered format and lint steps; run the test/coverage step only for pull requests.
- Grant only `contents: read`, use `pull_request` rather than `pull_request_target`, cancel superseded quality runs for the same branch or PR, and set a 10-minute job timeout.
- Install from lockfiles, use the pinned toolchains, cache npm downloads plus Cargo registry/git/build output without caching secrets or `node_modules`.

### Release workflow

- Trigger on `pull_request` closed for `master`; require `github.event.pull_request.merged == true`.
- Capture `github.event.pull_request.merge_commit_sha` once and use the full value for checkout, artifact name, manifest, tag target, and release identity.
- Serialize whole release runs with one `master-release` concurrency group, `queue: max`, and no cancellation so up to 100 rapid merges wait rather than replace each other.
- Build on Windows in a `contents: read` job, with Tauri action in build-only mode and `--bundles nsis`. Produce one x64 installer plus `release-manifest.json`, verify both exist, and upload them as a short-lived workflow artifact named with the full merge SHA.
- Publish in a dependent job that does not build or execute application source and alone receives `contents: write`. Use deterministic tag `release-<full-merge-sha>`.
- On first attempt, create a draft targeted to the merge SHA, ask GitHub to generate notes from the immediately previous published release (or the full included history for the first release), upload and verify the installer and manifest, then publish last.
- On retry, verify any published matching tag targets the same commit and carries the expected asset digest, then exit successfully. Resume a matching draft; fail on identity or digest conflicts. Never overwrite a verified published asset.
- Use release title `periScope <app-version> (PR #<number>, <short-sha>)`. The application version remains sourced from current project metadata; the SHA tag, not a forced version bump, provides per-merge uniqueness.

### Coverage boundary policy

- Frontend coverage includes maintained TypeScript. Declarations are excluded. DOM/Tauri calls may be wrapped at their boundary, but state transitions, conversion, validation, event handling, and error presentation remain covered.
- Rust coverage includes business and transformation logic. `build.rs`, generated schemas, the binary-only `main.rs`, and genuinely unexercisable Win32/Tauri entry points may be excluded with a documented regex and line-level rationale.
- Prefer extracting pure code from `main.ts`, `overlay.rs`, and `lib.rs` to excluding whole mixed-responsibility modules.
- Coverage evidence is retained only long enough for review and troubleshooting; release assets are retained with the release.

## Complexity Tracking

No constitution violations require justification.
