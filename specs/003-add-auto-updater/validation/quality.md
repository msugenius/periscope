# Final local quality evidence

Recorded: 2026-07-31

The following release-ready checks passed from the repository root at version `0.2.0`:

| Check | Result |
|---|---|
| `npm run format:check` | Passed (Prettier and `cargo fmt --check`) |
| `npm run lint` | Passed (TypeScript and Clippy with warnings denied) |
| `npm run test:coverage` | Passed |
| `scripts/release/Test-ReleasePipeline.ps1` | 6 groups passed, 0 failed |

Frontend: 20 tests passed; 87.31% line coverage. Rust: 30 tests passed; 82.82% line coverage. Both independently exceed the constitutional 80% line gate.

The existing Rust coverage command excludes native composition/platform boundaries matching `(main|lib|hotkeys|overlay)\.rs`; updater logic is not excluded. `updater.rs` is instrumented and its pure metadata/state/failure behavior is covered directly. Real networking and NSIS replacement remain controlled integration boundaries rather than unit-test substitutions.
