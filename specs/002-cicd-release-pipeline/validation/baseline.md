# CI/CD Baseline

**Captured**: 2026-07-31

## Toolchains

- Node.js: 24.18.0
- npm: 11.18.0
- Rust/Cargo: 1.97.0, stable MSVC
- Application version: 0.1.0

## Existing quality state

| Check | Baseline |
|-------|----------|
| Frontend production build | Passed: TypeScript and Vite build completed in approximately 3.2 seconds |
| Rust formatting | Passed: `cargo fmt --all -- --check` completed in approximately 1.5 seconds |
| Rust lint | Unavailable before this feature because the active toolchain did not include Clippy |
| Frontend tests | No frontend test runner or tests existed |
| Rust tests | Passed: 3 settings tests, 0 failures |
| Frontend coverage | No coverage provider or threshold existed |
| Rust coverage | `cargo-llvm-cov` was not installed and no threshold existed |
| Hosted CI/CD | No `.github/` workflows existed |

## Dependency baseline

Runtime dependencies before this feature:

- `@tauri-apps/api` 2.11.1
- Rust runtime dependencies already declared in `src-tauri/Cargo.toml`

The feature may add development-only tooling but must not add an application runtime dependency.

## Installer baseline

Existing local release bundles:

- NSIS: `periScope_0.1.0_x64-setup.exe`, 1,960,620 bytes
- MSI: `periScope_0.1.0_x64_en-US.msi`, 2,961,408 bytes

Only the x64 NSIS installer is in scope for automated publication.
