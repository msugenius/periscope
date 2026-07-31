# US1 detection evidence

Recorded: 2026-07-31

## Automated independent result

- `cargo test --locked --manifest-path src-tauri/Cargo.toml updater::tests` passed all updater state and metadata tests.
- `npx vitest run src/update-ui.test.ts src/app.test.ts` passed detection, escaping, ordering, recovery, and non-blocking-render tests.
- `npm run test:coverage:frontend` passed at 86.72% line coverage.
- `npm run test:coverage:rust` passed at 83.32% line coverage after adding direct transition and failure-classification coverage.

The native `SessionCore::begin_check` test accepts the first request and rejects every later request, including after an operational failure. `start_update_check` launches network work only when that transition returns `true`, so settings-window recreation reads the existing snapshot and produces zero additional automatic requests. The resulting per-process automatic request count is exactly one accepted check.

The metadata tests accept only a greater stable `MAJOR.MINOR.PATCH` version for `windows-x86_64` with non-empty notes and a full lowercase source commit. Equal, lower, prerelease, build-suffixed, leading-zero, malformed, incomplete, and incompatible metadata cannot produce an offer. Offline, timeout, rate-limit, and invalid-metadata results become safe terminal snapshots without raw diagnostics.

## Remaining controlled evidence

The current workstation was not used as an update target because a developer installation is already running. Real endpoint/installer behavior remains assigned to the isolated Windows validation in T019; no claim about actual replacement is made here.
