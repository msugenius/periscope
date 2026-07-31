# Contract: Quality Workflow

## Identity

- Workflow file: `.github/workflows/quality.yml`
- Workflow display name: `Quality`
- Required job/check name: `quality` / `Quality / quality`
- Default permissions: `contents: read`; all unspecified permissions are `none`

## Trigger contract

| Event | Filter | Required stages |
|-------|--------|-----------------|
| `push` | Every branch; tags excluded | Format → Lint |
| `pull_request.opened` | Base is `dev` or `master` | Format → Lint → Test/Coverage |
| `pull_request.reopened` | Base is `dev` or `master` | Format → Lint → Test/Coverage |
| `pull_request.synchronize` | Base is `dev` or `master` | Format → Lint → Test/Coverage |
| `pull_request.edited` | Base is `dev` or `master` | Format → Lint → Test/Coverage |

Pull requests targeting any other branch receive ordinary validation through pushes to their source branch; they do not receive the protected-target coverage gate.

## Execution contract

1. Checkout the event revision.
2. Install pinned Node/Rust tools and locked dependencies.
3. Restore dependency/build caches that contain no credentials.
4. Run `npm run format:check`.
5. Only on success, run `npm run lint`.
6. Only for an eligible PR and after lint succeeds, run `npm run test:coverage`.

The job fails when any applicable command exits nonzero. Later steps remain skipped. The overall job timeout is 10 minutes.

## Coverage contract

- Frontend and Rust are separate instrumented production codebases.
- Each must meet at least 80% line coverage.
- A passing combined average cannot compensate for one codebase below 80%.
- Exclusions are limited to declarations, generated files, build scripts, binary entry points, and documented platform-boundary code that cannot be exercised reliably.
- Coverage failures identify the codebase, measured percentage, and threshold.

## Concurrency contract

- Group by PR number for pull requests and full branch ref for pushes.
- Cancel an in-progress run when a newer revision enters the same group.
- Push and PR runs are distinct; a branch with an open protected-target PR may receive both results.

## Security contract

- Use `pull_request`, never `pull_request_target`, because repository code executes.
- Do not provide repository secrets or a write-capable token.
- Pin each external action to a reviewed full commit SHA and annotate its human-readable release.
- Never cache `node_modules`, secrets, tokens, release assets, or repository-local machine caches.

## Observable result

The check displays:

- Source branch/ref and commit SHA
- Applicable stages in required order
- Passed, failed, skipped, or cancelled stage states
- Actionable command diagnostics
- Separate frontend and Rust line-coverage results for PRs
- Total duration
