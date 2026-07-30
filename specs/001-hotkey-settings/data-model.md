# Data Model: Configurable Hotkeys

## AppSettings

The single persisted settings document loaded at startup and saved after accepted changes.

| Field group | Type | Rules |
|-------------|------|-------|
| crosshair | Existing crosshair settings | Validation and defaults remain unchanged |
| hotkeys | `HotkeySettings` | Defaults when absent so existing JSON files remain valid |

## HotkeySettings

The complete user-configurable hotkey pair. Updates are validated and committed as a unit.

| Field | Type | Default | Rules |
|-------|------|---------|-------|
| closeApp | canonical shortcut string | `F3` | Must parse to one non-modifier key plus optional supported modifiers; must differ from showSettings |
| showSettings | canonical shortcut string | `F4` | Must parse to one non-modifier key plus optional supported modifiers; must differ from closeApp |

### Validation

1. Both values are required and parseable.
2. Pure modifier combinations are invalid.
3. The two canonical shortcuts are distinct.
4. A proposed pair is not persisted until all changed system registrations succeed.
5. A missing hotkeys value in an older settings file resolves to the default pair.

## HotkeyAction

A stable identifier used by native dispatch and the UI.

| Identifier | Label | Effect |
|------------|-------|--------|
| closeApp | Close app | Mark the application as intentionally quitting and exit the process |
| showSettings | Show settings | Restore/focus the existing Settings window or create one |

The initial feature contains exactly these two values. The identifiers are persisted indirectly through fixed fields, not as user-created records.

## HotkeyRuntimeState

Native process-only state; it is not persisted.

| Field | Type | Purpose |
|-------|------|---------|
| registered | action-to-parsed-shortcut map | Identifies the active system registrations |
| pressed | set of currently pressed actions/shortcuts | Suppresses operating-system repeat until Released |
| recordingSuppressed | boolean or equivalent guard | Prevents actions from firing during UI capture |
| errors | action-to-message map | Reports startup or registration failures to Settings while keeping tray access available |

### State transitions

```text
Loaded -> Validated -> Registered
                     -> RegistrationFailed (error exposed; tray remains usable)

Registered -> RebindRequested -> Validated -> Registered(new) -> Persisted
                                      |              |
                                      `-> Rejected   `-> Failed -> RolledBack(old)

Released -> Pressed(action dispatched once) -> RepeatedPressed(ignored) -> Released
```

## Persistence compatibility

- Existing crosshair-only JSON: hotkeys default to F3/F4 on deserialize.
- Valid hotkey JSON: values are loaded, validated, and registered at startup.
- Malformed/unavailable saved value: visual settings still load, tray operations remain available, the Hotkeys page shows the error, and Reset hotkeys can recover to defaults.
