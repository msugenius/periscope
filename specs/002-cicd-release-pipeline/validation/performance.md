# Performance Validation

## Local implementation samples

These local Windows x64 samples validate command shape and provide a pre-hosted-run sanity check. They are not substitutes for GitHub-hosted cold/warm samples.

| Surface | Cache state | Sample result | Budget |
|---|---|---:|---:|
| Strict lint | Warm | 3.1 seconds | Branch target: 5 minutes |
| Frontend coverage | Warm | 3.6 seconds | PR target: 10 minutes |
| Full frontend + Rust tests | Warm | 27.8 seconds | PR target: 10 minutes |
| Rust coverage after instrumentation build | Warm | 13.0 seconds | PR target: 10 minutes |
| x64 NSIS release build | Cold target-specific build | 69.5 seconds | Release target: 20 minutes |

All local samples are comfortably below their corresponding workflow budgets. GitHub-hosted branch, PR, and release measurements— including cache hit/miss state and at least enough samples to evaluate the 90% target—remain pending until the workflows are committed and exercised.
