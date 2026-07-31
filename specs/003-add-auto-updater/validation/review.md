# Updater and release review

Recorded: 2026-07-31

## Result

No release-blocking local source finding remains after review.

- KISS/ownership: one Windows-only native updater dependency owns HTTP, signature verification, and installation. `updater.rs` owns process state; `update-ui.ts` owns presentation; `lib.rs` only composes commands/plugins; release rules are shared in one PowerShell module.
- Performance: Settings renders before updater subscription/network completion. The native transition accepts one automatic check per process and no polling or interval was found.
- Locking/resources: no mutex guard crosses an async check, download, or installer wait. Dismissal and terminal failures clear the retained native `Update`; installation consumes it before download. A missing pending object is rejected before changing phase.
- Untrusted data: release notes and safe messages enter the DOM through `textContent`; no remote HTML is injected. Native operational errors map to stable codes and summaries rather than raw response bodies, URLs, tokens, or paths.
- Least privilege: no JavaScript updater/process package or capability entry was added. Generated Tauri schemas describe the native plugin, but `capabilities/default.json` was not expanded. The publish job alone has `contents: write`; it does not check out source and receives no signing secret.
- Signing: only the public key is committed. The workflow references protected `release` environment secrets only on the trusted exact-merge build/sign step. Repository-file scanning found no private-key material and no interval polling.
- Release integrity: strict five-file SemVer agreement, numeric greatest-stable selection, exact merge/tag identity, deterministic private/public manifests, immutable version-tagged URLs, literal signature metadata, four-asset draft verification, publish-last/latest mutation, and conflict-without-overwrite behavior are enforced.
- Dependencies: production JavaScript dependencies are unchanged. The Rust updater plugin is Windows-only; its lockfile additions are the required native HTTP/signature stack.

## Evidence boundary

This review verifies source and local automation. It does not replace disposable GitHub publication, real signed NSIS replacement, performance measurements, or representative-user evidence; T019, T028, T029, T032, T034, and T035 remain open for those results.
