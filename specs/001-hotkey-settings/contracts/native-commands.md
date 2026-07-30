# Native Command Contract: Hotkey Settings

The frontend uses Tauri invoke commands. Command names and payload keys use the existing snake_case command and camelCase serialized-data conventions.

## `get_settings`

Existing command, extended response:

```json
{
  "enabled": true,
  "color": "#35E8FF",
  "opacity": 100,
  "length": 20,
  "thickness": 2,
  "gap": 6,
  "centerDot": true,
  "dotSize": 3,
  "tStyle": false,
  "outline": true,
  "outlineThickness": 1,
  "outlineColor": "#000000",
  "xOffset": 0,
  "yOffset": 0,
  "hotkeys": {
    "closeApp": "F3",
    "showSettings": "F4"
  },
  "hotkeyErrors": {}
}
```

`hotkeyErrors` is runtime status for display and is not written to the settings file.

## `update_hotkeys`

Request:

```json
{
  "hotkeys": {
    "closeApp": "Control+Shift+F3",
    "showSettings": "F4"
  }
}
```

Success response:

```json
{
  "closeApp": "Control+Shift+F3",
  "showSettings": "F4"
}
```

Behavior:

1. Parse and canonicalize both bindings.
2. Reject duplicates before changing registrations.
3. Apply changed registrations as one transaction.
4. Persist and return the accepted canonical pair only after success.
5. Temporarily suppress dispatch while applying a pair.

On error, return a readable string suitable for an inline Hotkeys-page message. The frontend keeps its last accepted pair and the native layer restores the previous active pair.

Expected error classes:

- invalid or unsupported shortcut
- duplicate shortcut across actions
- shortcut unavailable or reserved by the operating system or another application
- persistence failed, with registration rolled back to the previously persisted pair

## `reset_hotkeys`

Request: no payload.

Success response:

```json
{
  "closeApp": "F3",
  "showSettings": "F4"
}
```

Behavior is the same transaction as `update_hotkeys`, using defaults. Crosshair fields are preserved.

## `set_hotkey_recording`

Request:

```json
{
  "recording": true
}
```

Response: no data on success.

Behavior: sets a process-wide dispatch guard. While `recording` is true, Pressed events do not invoke Close app or Show settings, though Released events still clear pressed-state tracking. The UI sends `false` after capture, cancellation, navigation, or window teardown. Native registration and validation remain available while dispatch is suppressed.

## Existing commands

- `update_settings` continues to update visual crosshair fields and preserves the active hotkey pair.
- `reset_settings` continues to reset visual crosshair fields and preserves the active hotkey pair.
- `hide_settings` and `minimize_settings` remain unchanged.

## Native action contract

- `closeApp`: dispatch only on the first Pressed event before a matching Released event; mark intentional quit and exit.
- `showSettings`: dispatch only on the first Pressed event before Released; reuse the singleton show/restore/focus helper.
