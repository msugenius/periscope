# Repository Settings Validation

## Implementation boundary

Workflow and Dependabot configuration is implemented in the repository. Applying branch rules, token defaults, and immutable-release settings changes live GitHub administration state and is not performed implicitly by a source implementation run.

## Required administrator actions

| Surface | Required setting | Evidence status |
|---|---|---|
| `dev` ruleset | Require pull requests and `Quality / quality`; restrict bypass | Pending administrator application |
| `master` ruleset | Require pull requests, current branch, and `Quality / quality`; block force-push/delete/direct contributor pushes | Pending administrator application |
| Actions | Default token read-only; allow job-scoped `contents: write`; no PAT | Workflow scopes implemented; repository default pending review |
| Releases | Enable immutable releases where available | Pending administrator application |
| Dependabot | Review npm and pinned-action update PRs | Configured in `.github/dependabot.yml` |
| Installer trust | Record unsigned SmartScreen limitation | Documented in `README.md` |

After the workflows have run at least once, select the exact check context `Quality / quality` in both rulesets. A missing or failing check must block merge. The `Windows Release` build job must show `contents: read`; only its dependent publisher may show `contents: write`.
