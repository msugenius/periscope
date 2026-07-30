# UI Contract: Hotkeys Settings Page

## Navigation

- Add an enabled `Hotkeys` item to the existing sidebar.
- Selecting `Crosshair` renders the existing editor unchanged.
- Selecting `Hotkeys` renders the hotkey page and marks only that navigation item active.
- Re-rendering after save/reset preserves the current page.

## Page content

The page contains:

1. Heading: `Hotkeys` with concise help that shortcuts work globally while periScope runs.
2. One row for `Close app`, showing its accepted binding and default `F3`.
3. One row for `Show settings`, showing its accepted binding and default `F4`.
4. A `Reset hotkeys` action that affects only these two bindings.
5. An inline status/error region announced to assistive technology.

## Recording interaction

1. Activating a row's binding control enters recording mode for that row.
2. The control displays a prompt such as `Press shortcut...`.
3. The next non-modifier key plus currently held standard modifiers forms the proposal.
4. Escape cancels recording and restores the accepted display value.
5. Pure modifier presses do not complete recording.
6. While recording, native application shortcut dispatch is suppressed.
7. On accepted update, the canonical binding replaces the displayed value and an automatic-save confirmation appears.
8. On rejection, the prior accepted value remains and a readable inline error explains what to change.

## Keyboard and state behavior

- Binding controls are reachable and operable by keyboard.
- Recording focus remains visible.
- Only one row records at a time.
- Navigating away or closing Settings cancels recording.
- Duplicate proposals are rejected before invoking native registration when possible, while native validation remains authoritative.
- Startup registration errors appear next to the affected action with a Reset hotkeys recovery action.
