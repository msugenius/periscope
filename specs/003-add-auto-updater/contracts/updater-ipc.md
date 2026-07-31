# Contract: Native Updater Commands and Events

The native process owns updater state. Settings webviews render snapshots and send explicit user actions through custom commands.

## Snapshot shape

```json
{
  "phase": "available",
  "installedVersion": "0.1.0",
  "candidate": {
    "version": "0.2.0",
    "notes": "Release summary",
    "sourceCommit": "0123456789abcdef0123456789abcdef01234567",
    "platform": "windows-x86_64"
  },
  "downloadedBytes": null,
  "totalBytes": null,
  "failureCode": null,
  "message": null
}
```

Allowed phases are `idle`, `checking`, `up-to-date`, `available`, `dismissed`, `downloading`, `installing`, and `failed`.

## Commands

### `get_update_status`

- Input: none
- Output: current Update Session snapshot
- Behavior: read-only; never starts network work

### `start_update_check`

- Input: none
- Output: current snapshot immediately after accepting or deduplicating the request
- Behavior: starts one background check only from `idle`; every later call returns existing state without another automatic request
- Error handling: operational failures become `failed` state/events rather than rejecting application boot

### `dismiss_update`

- Input: `{ "version": "MAJOR.MINOR.PATCH" }`
- Output: `dismissed` snapshot
- Preconditions: phase is `available` and input version equals the retained candidate
- Behavior: drops the retained plugin update and suppresses another offer until process exit

### `install_update`

- Input: `{ "version": "MAJOR.MINOR.PATCH" }`
- Output: on failure, the terminal snapshot; on success, Windows installation exits and restarts the application before a normal response is required
- Preconditions: phase is `available`, input version equals the retained candidate, and no other operation is active
- Behavior: consumes the pinned candidate, emits download/install progress, relies on mandatory signature verification, and never substitutes a newer “latest” candidate

## Event

- Name: `periscope://updater-state`
- Payload: complete Update Session snapshot
- Emission: after every phase, progress, candidate, or failure transition
- Delivery: best effort to currently open settings webviews; native state remains authoritative when no webview exists

## Webview sequence

1. Render the normal settings shell.
2. Subscribe to `periscope://updater-state`.
3. Invoke `get_update_status` and render the snapshot.
4. Invoke `start_update_check` without awaiting it from the boot critical path.
5. On webview cleanup, unsubscribe event handlers; do not cancel native work.

Subscribing before the snapshot read prevents a transition gap. Rendering a complete snapshot makes duplicate/out-of-order DOM updates harmless.

## Error and trust rules

- Release notes and messages are untrusted text and must be escaped.
- User-facing failures expose stable codes and safe summaries, never private keys, tokens, raw response bodies, local paths, or full diagnostic URLs.
- Duplicate start/install/dismiss commands are idempotent or rejected without corrupting state.
- Only the native retained update object authorizes download/install; frontend fields are display/confirmation data.
