# US2 automated evidence

Recorded: 2026-07-31

## Result

- Native updater tests pass candidate/version pinning, dismissal, duplicate suppression, monotonic bounded progress, unknown content lengths, download/signature/install failures, safe recovery messages, and candidate cleanup.
- jsdom tests pass approve and dismiss actions, immediate duplicate-action disabling, accessible percentage progress, automatic Windows restart messaging, escaped release notes, and restart-to-retry guidance.
- Full frontend line coverage is 86.72% and full instrumented Rust line coverage is 83.32%; both project gates exceed 80%.
- `npm run lint` passes TypeScript checking and Rust Clippy with warnings denied.

`install_update` consumes the exact native `Update` object retained by detection after re-validating the displayed version. It never performs a second latest lookup. Tauri verifies the mandatory updater signature before the bytes reach the passive installer. Any terminal failure clears the retained native object and emits only a stable code plus safe recovery text.

## Boundary of this evidence

These tests do not execute NSIS replacement, elevation, restart, low-disk behavior, or concurrent installed instances. Those destructive scenarios remain in T019 and must run with disposable signed versions in an isolated Windows environment.
