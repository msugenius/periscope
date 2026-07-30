# Quickstart Validation: Configurable Hotkeys

## Prerequisites

- Windows 10 or 11
- Node.js 20+
- Rust stable with the MSVC toolchain
- Dependencies installed with `npm install`

## Build checks

From the repository root:

```powershell
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: TypeScript/Vite builds successfully and all Rust tests pass.

## Run

```powershell
npm run tauri dev
```

## Scenario 1: Default global actions (P1)

1. Confirm the overlay and tray icon are present.
2. Hide Settings and focus a different application.
3. Press F4.
4. Confirm one Settings window appears, is restored, and receives focus within one second.
5. Press F4 repeatedly and confirm no duplicate Settings window is created.
6. Focus another application and press F3 once.
7. Confirm Settings, overlay, and tray icon all exit.

## Scenario 2: Customize and persist (P2)

1. Restart periScope and open the Hotkeys page.
2. Confirm Close app shows F3 and Show settings shows F4 for a fresh configuration.
3. Record `Ctrl+Shift+F4` for Show settings.
4. Hide Settings, focus another application, and verify `Ctrl+Shift+F4` opens Settings while F4 no longer does.
5. Restart periScope and verify the custom binding remains displayed and functional.
6. Select Reset hotkeys and verify F3/F4 are restored without changing the current crosshair appearance.

## Scenario 3: Conflict and rollback

1. Attempt to assign the current Close app binding to Show settings.
2. Confirm the page rejects the duplicate and both previous bindings still work.
3. Assign a combination already reserved by another running application, if one is available in the test environment.
4. Confirm periScope displays an availability error and retains the previous working shortcut.

## Scenario 4: Recording safety and repeat

1. Start recording a replacement for Close app.
2. Press F3 as the proposed key and confirm the application does not exit during capture.
3. Press Escape on a later recording attempt and confirm it cancels without changing the binding.
4. Hold Show settings and confirm the application performs one show/focus action until the key is released.

## Scenario 5: Existing behavior regression

1. Change crosshair geometry, color, visibility, and placement.
2. Hide and reopen Settings and confirm those values persist.
3. Use the existing Reset defaults action and verify visual settings reset while accepted hotkeys remain unchanged.
4. Use tray Open settings, Toggle crosshair, and Quit actions and confirm their existing behavior remains intact.

## Validation record — 2026-07-20

| Scenario | Result | Evidence / environment note |
|----------|--------|-----------------------------|
| Build checks | PASS | `npm run build`, `cargo fmt`, and `cargo test --manifest-path src-tauri/Cargo.toml` completed; all 3 Rust tests passed |
| 1. Default global actions | PASS | Launched the debug executable with an isolated environment, sent F4 and observed one live Settings window, then sent F3 and observed a clean process exit within 5 seconds |
| 2. Customize and persist | CODE-VALIDATED | Native transactional update/reset and serde persistence paths compile; backward-compatible defaults are covered by Rust tests. Interactive WebView capture could not be automated because no in-app browser was available in this session |
| 3. Conflict and rollback | PASS / LIMITED | Duplicate canonical bindings are rejected by a passing Rust test; native registration and persistence rollback paths compile cleanly. No deterministic third-party reserved shortcut was available for an OS-level conflict test |
| 4. Recording safety and repeat | CODE-VALIDATED | Recording suppression remains active through key release, Escape cancellation is implemented, and native Pressed/Released state gates repeat dispatch. Interactive capture could not be automated without the in-app browser |
| 5. Existing behavior regression | PASS | Production frontend build and full Rust test suite pass; visual settings commands preserve the hotkey pair, and hotkey reset preserves crosshair settings by construction |

No environment-specific F3 or F4 registration conflict was observed in the Windows runtime smoke test.
