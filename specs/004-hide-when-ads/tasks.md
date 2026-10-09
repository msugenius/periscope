# Tasks: Hide when ADS

**Input**: [plan.md](plan.md), [spec.md](spec.md), research and contracts.
**Tests**: Automated behavior tests plus manual Windows input-chain verification; maintain the 80% instrumented coverage floor.

## Phase 1: Foundation

- [X] T001 [US2] Add defaulted `hide_when_ads` persistence and tests in `src-tauri/src/settings.rs` and `src-tauri/src/lib.rs`.

## Phase 2: User Story 1 - Hide during ADS (P1)

- [X] T002 [US1] Implement conditional mouse observation, pass-through, transition visibility, cleanup, and enable failure in `src-tauri/src/overlay.rs` and `src-tauri/Cargo.toml`.
- [X] T003 [US1] Connect runtime preference command, save rollback, and startup in `src-tauri/src/lib.rs`.

## Phase 3: User Story 2 - Configure and remember (P2)

- [X] T004 [US2] Add the Settings contract and Hotkeys toggle in `src/ui-model.ts`, `src/app.ts`, and `src/styles.css`.
- [X] T005 [US2] Test UI save success and failure in `src/app.test.ts`.

## Phase 4: Validation

- [X] T006 Run formatting, frontend and native tests, coverage, and a Windows build; record results and any platform verification limit.
- [X] T007 Audit implementation against `spec.md`, `plan.md`, and all tasks; append convergence work only if gaps remain.
