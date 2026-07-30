# Tasks: Configurable Hotkeys

**Input**: Design documents from `/specs/001-hotkey-settings/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: No dedicated test-authoring tasks are included because the feature specification did not request TDD. Build checks and end-to-end validation are included in the final phase.

**Organization**: Tasks are grouped by user story so the default global actions can be delivered as an MVP before customization UI.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel because it changes different files and has no incomplete dependency
- **[Story]**: Maps the task to a user story in `spec.md`
- Every task names its exact target file or validation artifact

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add the supported global-shortcut integration to the existing native project.

- [X] T001 Add the Windows desktop `tauri-plugin-global-shortcut` 2.x dependency and resolve its lockfile entries in `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Provide the persisted model and process-lifetime registration service used by both stories.

**CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 Add backward-compatible `HotkeySettings` defaults (F3/F4), canonical validation, duplicate rejection, and persistence composition to `src-tauri/src/settings.rs`
- [X] T003 Create the native hotkey controller with parsed action mappings, transactional register/rollback behavior, startup error state, dispatch suppression, and Pressed/Released repeat gating in `src-tauri/src/hotkeys.rs`
- [X] T004 Wire the global-shortcut plugin, hotkey controller, persisted settings, and runtime status into application setup and `AppState` in `src-tauri/src/lib.rs`

**Checkpoint**: The application can load, validate, register, and safely replace a two-action hotkey pair for the process lifetime.

---

## Phase 3: User Story 1 - Control periScope with global hotkeys (Priority: P1) MVP

**Goal**: F3 fully exits periScope and F4 opens/restores/focuses a singleton Settings window from anywhere, including when the WebView is absent.

**Independent Test**: Hide Settings, focus another application, press F4 and verify one focused Settings window appears; then press F3 and verify the window, overlay, tray icon, and process exit.

- [X] T005 [US1] Route the Close app shortcut through an intentional-quit helper shared with the tray Quit action in `src-tauri/src/lib.rs`
- [X] T006 [US1] Route the Show settings shortcut through the existing singleton create/restore/focus helper and make repeated invocations idempotent in `src-tauri/src/lib.rs`
- [X] T007 [US1] Complete default startup registration and graceful unavailable-binding behavior without blocking tray startup in `src-tauri/src/hotkeys.rs`

**Checkpoint**: User Story 1 is independently functional with the default shortcuts and no Settings UI changes required.

---

## Phase 4: User Story 2 - Review and customize hotkeys (Priority: P2)

**Goal**: Users can discover, record, validate, save, recover, and reset both bindings from a dedicated Hotkeys page.

**Independent Test**: Change Show settings from F4 to another valid combination, verify it activates immediately and survives restart, verify a duplicate is rejected without losing the prior binding, then reset to defaults without changing crosshair settings.

- [X] T008 [US2] Implement `get_settings` runtime status output plus transactional `update_hotkeys`, isolated `reset_hotkeys`, and `set_hotkey_recording` commands per `specs/001-hotkey-settings/contracts/native-commands.md` in `src-tauri/src/lib.rs`
- [X] T009 [US2] Extend the frontend settings types/defaults and add state-preserving Crosshair/Hotkeys sidebar routing and Hotkeys page markup per `specs/001-hotkey-settings/contracts/hotkeys-ui.md` in `src/main.ts`
- [X] T010 [US2] Implement accessible shortcut recording, modifier canonicalization, Escape cancellation, duplicate pre-checks, native suppression calls, save/reset invokes, and accepted-value rollback in `src/main.ts`
- [X] T011 [P] [US2] Add responsive Hotkeys page, binding-row, recording-focus, save-status, and inline-error styles consistent with the existing shell in `src/styles.css`
- [X] T012 [US2] Display startup registration errors beside the affected action and connect recovery to Reset hotkeys in `src/main.ts`

**Checkpoint**: User Stories 1 and 2 are functional; customization does not weaken the default global actions or alter crosshair settings.

---

## Phase 5: Polish & Cross-Cutting Concerns

**Purpose**: Verify compatibility, document behavior, and exercise failure paths across both stories.

- [X] T013 [P] Document default/custom hotkeys and the Hotkeys Settings page in `README.md`
- [X] T014 Run formatting, `npm run build`, and `cargo test --manifest-path src-tauri/Cargo.toml`, resolving any feature-introduced failures in `src/main.ts`, `src/styles.css`, and `src-tauri/src/`
- [X] T015 Execute every Windows scenario and record any environment-specific unavailable-shortcut observations in `specs/001-hotkey-settings/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: Starts immediately.
- **Foundational (Phase 2)**: Depends on T001 and blocks both user stories.
- **User Story 1 (Phase 3)**: Depends on T002-T004 and is the MVP.
- **User Story 2 (Phase 4)**: Depends on T002-T004; it may be implemented alongside US1 after the foundation, but final validation also exercises US1 action dispatch.
- **Polish (Phase 5)**: Depends on the stories selected for delivery.

### User Story Dependencies

- **User Story 1 (P1)**: No dependency on User Story 2. It works with persisted defaults and native actions alone.
- **User Story 2 (P2)**: Uses the foundational registration service and action identifiers, but its page and rebind flow can be implemented and tested against those contracts independently of US1 UI.

### Within Each User Story

- For US1, implement the shared quit path before binding Close app, then confirm singleton Settings behavior and startup fallback.
- For US2, expose native commands before connecting the UI; render navigation/page state before recording logic; apply error recovery after accepted-value handling exists.
- Run the complete quickstart only after build checks pass.

### Parallel Opportunities

- After T002-T004, US1 native action wiring and US2 UI planning can proceed independently.
- T011 changes only `src/styles.css` and can run in parallel with T008 in `src-tauri/src/lib.rs` or T009/T010 in `src/main.ts` once the UI contract is agreed.
- T013 changes only `README.md` and can run in parallel with final code validation.

## Parallel Example: User Story 2

```text
Task T008: Implement the native command contract in src-tauri/src/lib.rs
Task T011: Implement Hotkeys page styles in src/styles.css
```

T009 and T010 both modify `src/main.ts`, so they remain sequential even though their concerns differ.

## Implementation Strategy

### MVP First

1. Complete T001-T004.
2. Complete T005-T007.
3. Stop and validate the P1 scenario with F3/F4 while another application is focused.
4. Deliver the native default shortcuts if customization is deferred.

### Incremental Delivery

1. Setup and foundation establish safe registration and persistence.
2. User Story 1 delivers immediately useful default global actions.
3. User Story 2 adds discovery, customization, persistence, errors, and reset without changing the P1 defaults.
4. Polish validates build quality, regression behavior, and real Windows conflict handling.

## Notes

- All task IDs are sequential and every implementation task includes an exact file path.
- `[P]` is used only where tasks do not contend for the same file or incomplete behavior.
- Use `research.md`, `data-model.md`, and both files under `contracts/` as the implementation authority.
- Do not persist a proposed pair until all changed registrations succeed and the old pair can be restored on failure.
