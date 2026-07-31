# Quickstart results

Recorded: 2026-07-31

## Completed locally

- Prerequisites and locked dependencies: available.
- Section 1 quality/release rules: passed; see [quality.md](./quality.md).
- Section 2 version declarations: all five agree at `0.2.0`; mismatch rejection passes.
- Section 4 updater behavior: native/jsdom scenarios pass; see [us1-detection.md](./us1-detection.md) and [us2-automated.md](./us2-automated.md).
- Security/source review: passed; see [review.md](./review.md) and [signing-setup.md](./signing-setup.md).

## Not executed in this environment

- Section 3 disposable signed artifact build and Section 7 real replacement require a disposable signing key and isolated Windows installation (T019).
- Sections 5–6 require a disposable GitHub repository with protected release environment and controlled release states (T028–T029).
- Section 8 still needs matched performance/footprint measurements; current partial evidence is in [performance-footprint.md](./performance-footprint.md).

The production signing key was not exported for local validation, and the existing developer app process was left untouched.
