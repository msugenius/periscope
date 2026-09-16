# Updater Signing Setup

**Configured**: 2026-07-31

**Repair (2026-09-16)**: the `fix/key-mismatch` change introduced public key
`FB899533C1FA038C` with a second Base64 layer, preventing `v0.4.3` from being
signed. The first repair restored the `v0.4.2` public key (`E78508C2EF416E0D`)
to try to preserve compatibility with installed clients.

The signing preflight in [run 35082018929](https://github.com/msugenius/periscope/actions/runs/35082018929/job/104747999984)
then successfully decrypted the configured private key and signed a probe with
key ID `FB899533C1FA038C`. The configuration now uses that key's public value
from commit `daa6c1f`, with the extra Base64 layer removed. No private key or
password was generated or changed during this repair. The next release
preflight must verify the full signature against this corrected public key.
Existing `v0.4.2` installations require one manual installation because they
trust the previous public key.

For checked key generation, exact secret names, local verification, and the
release flow, see [Signing setup and release preparation](../../../README.md#signing-setup-and-release-preparation).

## Trust material

- A password-protected Tauri updater keypair was generated for the stable Windows update channel.
- Only the replacement public key is committed in `src-tauri/tauri.conf.json`.
- The private key and password are stored as `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets in the GitHub `release` environment.
- The `release` environment allows deployments only from protected branches.
- Temporary private/public key files used during provisioning were removed from `src-tauri/target` after the secrets were stored.

## Rotation record

The initial provisioning attempt produced a weak password because an unsupported PowerShell RNG call failed. That keypair and both environment-secret values were immediately replaced using `RandomNumberGenerator.Create().GetBytes`; the weak private key and password are no longer configured or trusted. The committed public key belongs only to the replacement keypair.

## Custody and recovery

- Never copy the private key or password into repository files, logs, workflow artifacts, caches, validation fixtures, or pull-request secrets.
- Keep the GitHub environment secrets as the operational source of truth and restrict the signing job to trusted merged release-branch source.
- Loss of either secret prevents updates to installations that trust this public key. Recovery requires restoring the exact secret values; generating another key alone cannot update existing installations.
- Any intentional future rotation requires a separately planned transition that distributes trust for the next public key before retiring this one.
