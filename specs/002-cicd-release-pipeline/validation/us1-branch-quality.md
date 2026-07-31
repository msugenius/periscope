# User Story 1 Validation: Branch Quality

## Fail-first acceptance matrix

Before `.github/workflows/quality.yml` existed, no branch push produced a shared formatting or lint result. The following cases therefore failed by absence before implementation.

| Scenario | Trigger | Expected stages | Expected result |
|----------|---------|-----------------|-----------------|
| Clean ordinary branch | Push a formatted, lint-clean commit | Format → Lint | Pass |
| Formatting failure | Push an intentionally unformatted maintained file | Format; Lint skipped | Fail with file diagnostics |
| Lint failure | Push formatted code with a deterministic TypeScript or Clippy failure | Format → Lint | Fail at lint |
| Slash branch | Push `ci/branch-validation` | Format → Lint | Same result as any other branch |
| Tag only | Push a tag without a branch update | None | No Quality workflow run |
| Superseded branch revision | Push twice to the same branch | Latest run replaces older active run | Older run cancelled |

## Static implementation evidence

- Workflow: `.github/workflows/quality.yml`
- Branch selector: `push.branches: ["**"]`
- Tag selector: omitted, so tag-only pushes do not match
- Stage order: checkout/setup/cache/install → format → lint
- Failure behavior: default step failure prevents later steps
- Permissions: `contents: read`
- Timeout: 10 minutes
- Concurrency: event plus full ref, with `cancel-in-progress: true`

## Hosted evidence

Hosted run links and observed durations require pushing disposable commits to GitHub. Record them here after the repository-local implementation is reviewed and pushed:

| Scenario | Run URL | Format | Lint | Duration | Notes |
|----------|---------|--------|------|----------|-------|
| Clean ordinary branch | Pending | Pending | Pending | Pending | Requires hosted run |
| Formatting failure | Pending | Pending | Expected skipped | Pending | Requires disposable commit |
| Lint failure | Pending | Pending | Pending | Pending | Requires disposable commit |
| Slash branch | Pending | Pending | Pending | Pending | Requires disposable branch |
| Tag only | Pending | N/A | N/A | N/A | Confirm no run is created |
