# Feature Specification: Configurable Hotkeys

**Feature Branch**: `[001-hotkey-settings]`

**Created**: 2026-07-20

**Status**: Draft

**Input**: User description: "Add support of hotkeys and add a related tab in the settings UI. Initially support Close app (default F3) and Show settings (default F4)."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Control periScope with global hotkeys (Priority: P1)

As a user, I can exit periScope or bring its Settings window forward from anywhere by pressing the configured hotkey, including while the Settings window is hidden or another application is focused.

**Why this priority**: The shortcuts deliver the feature's primary value and must work independently of the settings interface.

**Independent Test**: Start periScope with its Settings window hidden, focus another application, press F4 and verify Settings opens and receives focus; then press F3 and verify periScope and its overlay fully exit.

**Acceptance Scenarios**:

1. **Given** periScope is running and Settings is hidden, **When** the user presses F4, **Then** the Settings window opens and receives focus.
2. **Given** periScope is running and Settings is already visible or minimized, **When** the user presses F4, **Then** the existing Settings window is restored and receives focus without opening a duplicate window.
3. **Given** periScope is running, **When** the user presses F3, **Then** the entire application exits and the overlay and tray icon disappear.
4. **Given** the user has not changed any bindings, **When** periScope starts, **Then** Close app is bound to F3 and Show settings is bound to F4.

---

### User Story 2 - Review and customize hotkeys (Priority: P2)

As a user, I can open a Hotkeys tab in Settings, see each action and its current binding, change a binding, and have valid changes remain in effect after restarting periScope.

**Why this priority**: Customization prevents conflicts with games and other software while making the shortcuts discoverable, but the default shortcuts already provide a useful P1 increment.

**Independent Test**: Open the Hotkeys tab, change Show settings from F4 to another valid key, hide Settings, verify the new key opens Settings and F4 no longer does, restart periScope, and verify the new binding still works.

**Acceptance Scenarios**:

1. **Given** Settings is open, **When** the user selects the Hotkeys tab, **Then** the page lists Close app and Show settings with their current bindings.
2. **Given** the Hotkeys tab is open, **When** the user records a valid unused binding for an action, **Then** the new binding becomes active and is saved automatically.
3. **Given** a custom binding was saved, **When** periScope is restarted, **Then** the saved binding is displayed and active.
4. **Given** a user has customized one or both bindings, **When** the user resets hotkeys to defaults, **Then** Close app returns to F3 and Show settings returns to F4.

### Edge Cases

- A proposed binding matches the other periScope action's binding; the change is rejected and the previous valid binding remains active.
- A proposed binding cannot be registered because it is reserved or already owned by another application; the user sees a clear error and the previous valid binding remains active.
- A saved binding is malformed or can no longer be registered at startup; periScope remains usable, reports the affected binding in the Hotkeys tab, and offers recovery to defaults.
- A hotkey is held down or auto-repeats; one physical press triggers the associated action only once.
- Show settings is triggered repeatedly while Settings is opening; at most one Settings window exists.
- The Settings window is focused while a new binding is being recorded; the recording keystroke changes the binding without also running its normal application action.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide system-wide hotkey bindings for the Close app and Show settings actions while periScope is running.
- **FR-002**: The default Close app binding MUST be F3.
- **FR-003**: Triggering Close app MUST fully terminate periScope, including the overlay, Settings window, and tray icon.
- **FR-004**: The default Show settings binding MUST be F4.
- **FR-005**: Triggering Show settings MUST create or reveal, restore, and focus a single Settings window.
- **FR-006**: Settings MUST include a navigable Hotkeys tab that lists every supported hotkey action and its current binding.
- **FR-007**: Users MUST be able to record and save a replacement binding for each supported action.
- **FR-008**: Valid changed bindings MUST become active without requiring an application restart and MUST persist across restarts.
- **FR-009**: The system MUST prevent the two supported actions from using the same binding.
- **FR-010**: If a proposed binding is invalid or unavailable, the system MUST retain the previous working binding and explain the problem in the Hotkeys tab.
- **FR-011**: Users MUST be able to reset all hotkey bindings to their defaults.
- **FR-012**: Recording a new binding MUST suppress application hotkey actions until the recording interaction completes or is cancelled.
- **FR-013**: Repeated key events from holding a hotkey MUST NOT cause the action to execute more than once per physical key press.
- **FR-014**: Existing crosshair settings and behavior MUST remain unchanged when hotkeys are added or reset.

### Key Entities

- **Hotkey Binding**: The association between a supported application action and a user-visible key combination, including its default, saved value, active registration state, and any registration error.
- **Hotkey Action**: A stable application operation that can be invoked by a binding; initially Close app and Show settings.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of acceptance tests, the default F3 and F4 shortcuts perform their documented actions while another application is focused.
- **SC-002**: Settings becomes visible and focused within 1 second of a valid Show settings hotkey press under normal desktop load.
- **SC-003**: A user can find the Hotkeys tab, change either binding, and verify it is active in under 30 seconds.
- **SC-004**: In 100% of conflict and unavailable-binding tests, periScope retains a working prior binding and presents a recoverable error instead of silently losing the action.
- **SC-005**: In 100% of restart tests, valid customized bindings remain selected and functional after periScope restarts.

## Assumptions

- Hotkeys are system-wide Windows shortcuts and remain active when Settings is hidden or another application has focus.
- A binding consists of one non-modifier key with optional standard modifier keys; mouse buttons and multi-step sequences are outside the initial scope.
- F3 and F4 are defaults only and can be reassigned through the Hotkeys tab.
- Close app means a full process exit, unlike the existing Hide settings action.
- Hotkey changes use the existing automatic-save behavior of Settings.
- The initial release supports exactly two hotkey actions; profiles, per-application bindings, and enable/disable toggles are outside scope.
