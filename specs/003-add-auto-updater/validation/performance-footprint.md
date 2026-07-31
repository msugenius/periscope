# Performance and footprint evidence

Status: incomplete — controlled measurement environment required.

Recorded: 2026-07-31

- Automatic request count: source and transition tests prove one accepted check per native process and no polling; settings-webview recreation produces no new accepted check.
- Startup critical path: jsdom proves the normal Settings shell renders while `get_update_status` is still unresolved. This is structural evidence, not a p95 wall-clock measurement.
- Timeout budget: native checks use a 10-second request timeout.
- Idle work: no interval, scheduler, animation loop, or retry timer was added by the updater.
- Dependency footprint: no production JavaScript dependency was added; one Windows-only Rust updater plugin and its locked transitive stack were added.

Still required for T032: matched control/enabled p95 first-render samples, normal network check latency distribution, post-check CPU/memory, temporary-file cleanup after real success/failure, and signed installer/installed-size delta from the 1,968,509-byte baseline. The currently running developer installation was intentionally not replaced or used as the measurement target.
