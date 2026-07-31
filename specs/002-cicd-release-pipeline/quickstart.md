# Quickstart: Validate CI/CD and Windows Releases

This guide validates the implementation locally and through controlled GitHub scenarios. It does not replace branch-protection review or release permission setup.

## Prerequisites

- Windows x64 with Tauri's Microsoft C++ build prerequisites and WebView2
- Node.js 24 and npm
- Rust 1.97.0 with `rustfmt`, `clippy`, and `llvm-tools-preview`
- `cargo-llvm-cov`
- Repository access that can create branches and pull requests
- Maintainer access for rulesets and the release scenarios

Install locked project dependencies from the repository root:

```powershell
npm ci
rustup show
cargo llvm-cov --version
```

Expected: npm uses `package-lock.json`; the pinned Rust toolchain and required components are active; `cargo-llvm-cov` reports its installed version.

## 1. Run the same quality gates locally

```powershell
npm run format:check
npm run lint
npm run test:coverage
```

Expected:

- Formatting completes before linting.
- TypeScript checking and Rust Clippy report no errors or warnings promoted to errors.
- Frontend and Rust tests pass.
- Each coverage report shows at least 80% line coverage independently.
- Generated, declaration, build, and platform-boundary exclusions match the documented narrow policy in [plan.md](./plan.md).

To confirm fail-fast behavior, make a reversible formatting-only change in a disposable branch and rerun `npm run format:check`. Expected: formatting fails with the affected file; do not commit the intentional failure.

## 2. Build the selected Windows bundle locally

```powershell
npm run tauri -- build --bundles nsis
```

Expected:

- Build succeeds without changing application runtime dependencies.
- Exactly one intended x64 NSIS installer is selected from `src-tauri/target/release/bundle/nsis/`.
- MSI and portable-binary publication are not required.

The initial installer is unsigned; Microsoft SmartScreen may warn. Signing is outside this feature.

## 3. Validate an ordinary branch push

1. Push a disposable branch whose name includes a slash, such as `ci/branch-validation`.
2. Open the `Quality` workflow run for the pushed SHA.
3. Verify format runs before lint and tests are skipped for the `push` event.
4. Push a formatting failure to the disposable branch and verify lint is skipped after format fails.

Expected: every branch push receives a visible overall quality result within the 5-minute target under normal GitHub availability.

## 4. Validate pull-request gates

Repeat the scenario for pull requests targeting `dev` and `master`:

1. Open a PR and inspect the stable `Quality / quality` check.
2. Update the PR and verify the superseded PR run is cancelled and the newest revision runs.
3. Verify format, lint, and coverage tests run in order.
4. Introduce a controlled test or coverage failure and verify the check blocks merge.
5. Retarget a disposable PR to a non-protected branch and verify PR coverage is not required there.

Expected: eligible PRs cannot satisfy the required check unless both codebases pass the 80% line-coverage gate.

## 5. Verify repository settings

Apply and review the settings in [contracts/repository-settings.md](./contracts/repository-settings.md):

- `dev` and `master` require `Quality / quality`.
- Pull requests are required before merge.
- Workflow default permissions are read-only.
- GitHub Actions may create releases only through the release publisher job.

Expected: a failing or missing required check prevents merge to either protected branch.

## 6. Validate non-release paths

Using disposable changes:

1. Close a PR to `master` without merging.
2. Push directly to `master` only in a controlled test repository or temporary clone with equivalent settings.
3. Cancel a release build before publication.

Expected: none of these paths publishes a release. A cancelled build may leave a short-lived workflow artifact but no public GitHub release.

## 7. Validate the first successful release

Merge a fully passing PR into `master`.

Expected:

- The release workflow checks out the event's exact full merge SHA.
- The read-only build job produces one NSIS installer and a matching manifest.
- The publisher creates tag `release-<full-merge-sha>`.
- Release title contains the application version, PR number, and short SHA.
- Installer and manifest are attached and their size/digest match.
- GitHub-generated notes identify included changes and contributors.
- The public release appears only after verification and within the 20-minute target under normal availability.

Compare the release with [contracts/release-workflow.md](./contracts/release-workflow.md) and [contracts/release-manifest.md](./contracts/release-manifest.md).

## 8. Validate later releases, retries, and rapid merges

1. Merge another passing PR and confirm its generated notes start from the preceding published tag.
2. Rerun the completed workflow for the same merge SHA.
3. In a controlled test repository, merge two passing PRs close together.

Expected:

- Each merge yields one deterministic release and the correct installer.
- A rerun verifies the existing release and exits successfully without duplication.
- Rapid releases queue instead of cancelling; each changelog range and artifact remains associated with its own merge.
- A mismatched tag target or asset digest fails loudly and never overwrites a verified published release.

## 9. Record performance and footprint evidence

For representative warm-cache and cold-cache runs, record:

- Branch quality duration
- PR quality duration
- Release duration from merge to publication
- Cache hit/miss state
- Installer size
- Application runtime dependency diff

Expected: at least 90% of observed normal runs meet the 5/10/20-minute budgets, and the installed application's runtime work and dependency footprint are unchanged.
