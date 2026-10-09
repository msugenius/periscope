# Implementation Plan: Hide when ADS

**Date**: 2026-10-09 | **Spec**: [spec.md](spec.md)

## Summary

Add an independent persistent preference and a Hotkeys-page toggle. The native overlay message loop installs a low-level Windows mouse observer only while enabled; button transitions alter overlay visibility and always pass input onward.

## Technical Context

**Language/Version**: Rust 2024, TypeScript 7
**Primary Dependencies**: Tauri 2, existing windows-sys 0.61
**Storage**: Existing settings.json
**Testing**: Vitest and cargo test; manual Windows input check
**Coverage Tooling**: `npm run test:coverage`
**Target Platform**: Windows desktop
**Project Type**: Tauri desktop app
**Performance Goals**: Visibility within 50 ms; no polling, redraw, or observer while disabled
**Constraints**: No new dependency, preserve mouse event chain
**Scale/Scope**: One boolean, one UI control, native overlay module

## Constitution Check

- **Dead simple**: Reuse existing overlay loop and persistence; no background worker.
- **Performance first**: Observe only while enabled, change visibility on transitions, compare manual press latency with baseline overlay toggle.
- **Lightweight**: No polling or new dependency; release hook when disabled or the overlay ends.
- **Modular**: Overlay owns transient hold state and Windows input; settings owns preference; UI owns interaction.
- **KISS and DRY**: Visibility computed in one overlay location; preference stored once.
- **Test quality**: Test serialisation and UI state automatically; manual Windows event chain check covers platform boundary. Existing native coverage exclusion for overlay remains documented.

## Project Structure

```text
specs/004-hide-when-ads/{spec,plan,research,data-model,quickstart,tasks}.md
specs/004-hide-when-ads/contracts/settings.md
src/{app.ts,app.test.ts,ui-model.ts,styles.css}
src-tauri/src/{settings.rs,lib.rs,overlay.rs}
src-tauri/Cargo.toml
```

## Design

The preference is a top-level AppSettings boolean with a serde default. A dedicated command changes observer state, persists the value, and rolls back the observer on save failure. `get_settings` returns it with other settings. The overlay receives synchronous enable/disable requests on its own message loop, allowing observer installation errors to propagate. The observer callback only checks right-button transitions, updates visibility on that loop, and always calls the next observer. It does not rasterize on press. On release it redraws the latest settings. The startup observer is installed if the saved preference is on.

## Complexity Tracking

No constitution violations or new dependencies. A low-level mouse observer is required because global hotkey registration does not expose a held mouse button while preserving other applications' input.

## Post-Design Constitution Check

Passed: the design remains event-driven, module-local, and uses existing Windows bindings. Performance and coverage verification are tasks below.
