# Final Local Validation

## Completed locally

| Validation | Result |
|---|---|
| Locked npm install | Passing |
| Format check | Passing |
| TypeScript + strict Clippy lint | Passing |
| Frontend tests | 12 passing |
| Rust tests | 14 passing |
| Frontend line coverage | 84.81% |
| Rust measured line coverage | 85.71% |
| Frontend production build | Passing |
| x64 NSIS bundle | Passing; exactly one installer |
| Workflow lint | Passing except actionlint v1.7.12's lag behind GitHub's documented `concurrency.queue`; all other diagnostics clean |
| Runtime dependency audit | No new npm production package or Rust crate |
| Secret scan of source/workflow diff | No credential-like patterns found |

## Hosted scenarios still required

Branch event filters, ordered stage skip states, PR cancellation and branch protection, repository permission settings, generated release notes, draft retry, release conflicts, immutable releases, and hosted performance budgets require committed workflow execution or administrator access. Their evidence tables remain explicitly pending rather than containing simulated links or claims.
