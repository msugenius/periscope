# Pre-Feature Updater Baseline

**Captured**: 2026-07-31
**Source version**: 0.1.0 before updater registration or runtime integration

## Runtime and dependency baseline

| Measure | Baseline | Evidence |
|---|---:|---|
| Automatic updater requests per process | 0 | No updater dependency was registered and no source path performed release/network checks |
| Periodic updater polling | None | No updater module, timer, or network loop existed |
| npm production dependencies | 1 | `@tauri-apps/api` only in `package.json` |
| Rust updater dependencies | 0 | Pre-change `Cargo.toml` contained only Tauri, serde, global-shortcut, and Windows dependencies |
| x64 NSIS installer | 1,968,509 bytes | `specs/002-cicd-release-pipeline/validation/footprint.md` |
| Installer SHA-256 | `708ca6da70a21bd14acdddb6407dc73981ecf7234a463fb23d8ee8095878ba7a` | Same recorded release-build evidence |

## Timing and resource baseline

The previous validation recorded a 69.5-second cold target-specific x64 NSIS build. It did not instrument first-render latency or process idle CPU/working-set memory. Because T001 changes only the locked dependency graph and the updater is not registered until T005, the application runtime remains the valid pre-feature control through T004.

Final measurement must compare otherwise identical builds and record:

- p95 time from process start to the first interactive settings render;
- idle CPU and working-set memory after a five-minute quiescent period;
- request count from launch through terminal update-check state;
- final installed/installer size against the 1,968,509-byte baseline.

Missing historical first-render and idle samples are not treated as zero. T032 must capture both control and updater-enabled samples with the same harness before accepting the performance budgets.
