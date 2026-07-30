# Phase 0 Research: Configurable Hotkeys

## Decision 1: Own shortcut registration in the Rust process

**Decision**: Add Tauri's official Rust `tauri-plugin-global-shortcut` 2.x plugin and register/dispatch shortcuts from the native application lifecycle.

**Rationale**: periScope destroys the Settings WebView when it is hidden. A JavaScript handler would disappear with that window, while a Rust plugin handler remains alive with the tray/overlay process. The official plugin supports Windows and emits Pressed and Released states, which also enables one-action-per-press gating. See [Tauri Global Shortcut](https://v2.tauri.app/plugin/global-shortcut/) and the [Rust crate API](https://docs.rs/tauri-plugin-global-shortcut/latest/tauri_plugin_global_shortcut/).

**Alternatives considered**:

- JavaScript plugin bindings: rejected because callbacks are tied to a WebView that periScope intentionally destroys.
- Direct Win32 `RegisterHotKey`: rejected because the official Tauri plugin already supplies lifecycle integration, parsing, and registration errors.
- Local window key events: rejected because they do not work while another application is focused.

## Decision 2: Use a transactional native rebind operation

**Decision**: Send the complete proposed hotkey pair to one native update command. Validate syntax and uniqueness first, unregister only changed old bindings, register the proposed changed bindings, and persist only after all registrations succeed. If any registration fails, unregister newly added bindings and restore the previous pair before returning a user-facing error.

**Rationale**: The official API reports registration failure, and shortcuts may be owned by another application. Treating the pair as one transaction prevents a partial save where one action silently loses its shortcut.

**Alternatives considered**:

- Persist before registration: rejected because an unavailable shortcut would be stored as though it worked.
- Update each row independently without rollback: rejected because a second failure can leave the application with a partially applied pair.
- Register the new pair before unregistering old bindings: rejected because unchanged/current registrations can conflict with the application itself.

## Decision 3: Keep persisted configuration backward compatible

**Decision**: Extend the current JSON document with a hotkey settings value that has serde defaults of F3 and F4. Separate visual-reset and hotkey-reset operations so resetting hotkeys does not alter the crosshair.

**Rationale**: Existing settings files contain only crosshair values. Default-on-missing fields provide a migration-free upgrade, and separated reset commands satisfy the requirement that hotkey reset leave current crosshair configuration unchanged.

**Alternatives considered**:

- A second hotkeys file: rejected because it adds lifecycle and atomicity complexity for two fields.
- A one-time schema migration: rejected because serde defaults are sufficient and safer for this additive change.
- Make the existing Reset defaults button reset everything: rejected because that would unexpectedly alter unrelated settings.

## Decision 4: Record canonical combinations in the Settings UI

**Decision**: The Hotkeys page enters a temporary recording mode for one action, listens for one non-modifier key plus optional Ctrl/Alt/Shift/Meta modifiers, uses Escape to cancel, formats a canonical display/transport string, and sends the whole pair to the native command. Native parsing remains authoritative.

**Rationale**: Recording avoids free-text spelling errors while preserving common global shortcut combinations. Suppressing normal shortcut dispatch during recording prevents F3/F4 from closing or reopening the application as the user captures a replacement.

**Alternatives considered**:

- Free-text fields: rejected because invalid spellings and inconsistent modifier order are easy to create.
- Dropdowns for every key and modifier: rejected as slower and more visually complex for two bindings.
- Frontend-only validation: rejected because only native registration can establish actual system availability.

## Decision 5: Reuse existing app actions and window singleton behavior

**Decision**: Route hotkey dispatch through the same native `show_settings` and controlled quit paths used by the tray. Show settings restores/focuses an existing `main` window or creates it when absent; close sets the quitting flag before exiting.

**Rationale**: One action path prevents tray and shortcut behavior from diverging and preserves the existing single-window and exit-request rules.

**Alternatives considered**:

- Emit frontend events for actions: rejected because no frontend exists when Settings is hidden.
- Duplicate window/quit logic inside the handler: rejected because duplicated lifecycle logic can drift.
