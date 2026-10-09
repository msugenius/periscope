# Feature Specification: Hide when ADS

**Created**: 2026-10-09
**Status**: Ready
**Input**: Add a toggle in Hotkeys settings named "Hide when ADS". While enabled, holding the right mouse button hides the crosshair without interfering with other applications receiving the click.

## User Scenarios & Testing

### User Story 1 - Hide crosshair during ADS (Priority: P1)

With the option enabled, I can hold the right mouse button in another application and see the crosshair disappear until I release it.

**Independent Test**: Enable the option, focus an application that responds to right clicks, hold and release the button. Verify the crosshair hides and returns and the application still receives the click.

**Acceptance Scenarios**:

1. Given the crosshair is enabled and Hide when ADS is on, when right mouse button goes down, then the crosshair disappears for the full hold.
2. Given the button is held, when it goes up, then the crosshair reappears with its current settings.
3. Given Hide when ADS is off, when right mouse button is held, then the crosshair remains visible.
4. Given another application is focused, when the button is pressed and released, then that application receives both events normally.

### User Story 2 - Configure and remember the option (Priority: P2)

I can turn Hide when ADS on and off in Hotkeys settings and have that choice remembered after restart.

**Independent Test**: Turn on the option, restart periScope, verify it is still on and works; turn it off and verify the crosshair remains visible during right-button holds.

**Acceptance Scenarios**:

1. Given a fresh or older settings file, when Settings opens, then Hide when ADS is off.
2. Given the user changes the toggle, when the app restarts, then the chosen value and behavior persist.
3. Given the crosshair was manually disabled, when the button is released, then it stays disabled.

### Edge Cases

- Turning the option on while the button is already held immediately hides the crosshair.
- Turning it off while held immediately restores the crosshair if it is enabled.
- Other crosshair changes or preset selection during a hold do not make it visible until release.
- If mouse observation cannot be enabled, the toggle reports an error and retains the prior setting.
- Repeated button-down events do not cause repeated rendering.

## Requirements

### Functional Requirements

- **FR-001**: Hotkeys settings MUST show a toggle labeled "Hide when ADS", off by default.
- **FR-002**: When enabled, holding the right mouse button MUST hide the crosshair, and releasing it MUST restore only an otherwise enabled crosshair.
- **FR-003**: The right mouse button MUST continue to work normally in all other applications.
- **FR-004**: The choice MUST persist across restarts and older settings files MUST default it to off.
- **FR-005**: Switching the choice while the button is held MUST update visibility immediately.
- **FR-006**: If enabling observation fails, the previous setting MUST remain in effect and the user MUST see an error.

### Test Requirements

- **TR-001**: Automated tests cover defaulting, persistence, UI command and error state, and visibility decision.
- **TR-002**: Manual Windows verification covers event pass-through to another application and press/release behavior.
- **TR-003**: Native platform hook internals may be excluded from line coverage because they require live Windows input; keep instrumented production codebases at 80% or more.

### Performance and Footprint Requirements

- **PF-001**: Press/release should change visibility within 50 ms under normal desktop load.
- **PF-002**: Disabled mode must have no mouse observation; enabled mode must do no polling or continuous redraw, and repeat downs must do no redraw.
- **PF-003**: Add no runtime dependency or persistent asset; each mouse event must return promptly to the application input chain.

### Key Entities

- **Hide when ADS preference**: Persistent boolean, independent of hotkey bindings and crosshair presets.
- **ADS hold state**: Transient right-button state that affects visibility only while the option is enabled.

## Success Criteria

### Measurable Outcomes

- **SC-001**: All press/release cycles hide and restore the enabled crosshair within 50 ms in manual Windows testing.
- **SC-002**: All test clicks reach the focused application with the option on.
- **SC-003**: The saved choice survives restart; old settings load with the toggle off.
- **SC-004**: Disabled mode has zero mouse-observation work; enabled mode has zero polling or repeat redraws.
- **SC-005**: Automated line coverage remains at least 80% for each instrumented production codebase.

## Assumptions

- ADS means holding the right mouse button globally, regardless of which application has focus.
- Resetting hotkey bindings does not reset the independent ADS preference.
- The setting applies to the current Windows desktop session; elevated or isolated desktops are outside scope.
