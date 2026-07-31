# Updater Signing Setup

**Configured**: 2026-07-31

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
