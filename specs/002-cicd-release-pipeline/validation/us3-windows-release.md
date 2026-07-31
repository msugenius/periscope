# User Story 3 Validation: Windows Release

## Fail-first acceptance matrix

| Scenario | Expected workflow behavior | Required evidence |
|---|---|---|
| PR closed without merge | Build and publisher are skipped | Event link and skipped jobs |
| Direct push to `master` | No Windows Release run | Commit link and absence of release run |
| Merged PR to `master` | Build exact `merge_commit_sha` | Event SHA, checkout log, manifest SHA |
| Failed or cancelled build | Publisher never starts | Job dependency state |
| Installer cardinality | Exactly one non-empty x64 `*-setup.exe` | File name, size, validation log |
| Manifest integrity | Schema v1 fields and recomputed SHA-256/size agree | Manifest and publisher log |
| First release | Generated notes without a previous tag | Release link and notes |
| Later release | Notes start at immediately previous published release | Previous tag and comparison link |
| Same-SHA rerun | Matching published release is a no-op success | Rerun link and unchanged assets |
| Interrupted draft | Matching draft resumes and publishes last | Draft/retry run links |
| Conflicting tag or asset | Publisher fails without overwrite | Diagnostic and unchanged release |
| Two rapid merges | FIFO `master-release` queue preserves association | Both run links and release order |
| Permission boundary | Build is read-only; publisher alone has `contents: write` | Job permission logs |
| Performance | Normal merged release finishes within 20 minutes | Cold/warm durations |

## Local static evidence

The workflow implementation can be inspected locally for its event filter, exact-SHA identity, pinned actions, build/publish job permissions, manifest checks, deterministic tag, draft-first publication, and retry conflict handling. Execution links remain pending until the workflow is committed and exercised on GitHub.
