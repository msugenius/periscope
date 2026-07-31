# Contract: Release Manifest

The read-only Windows build job emits `release-manifest.json` beside the NSIS installer. The write-capable publisher treats it as untrusted input and validates every field against the workflow event and downloaded file.

## JSON shape

```json
{
  "schemaVersion": "1",
  "repository": "owner/name",
  "pullRequestNumber": 123,
  "mergeSha": "0123456789abcdef0123456789abcdef01234567",
  "applicationVersion": "0.1.0",
  "artifactName": "periScope_0.1.0_x64-setup.exe",
  "artifactSha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "artifactSizeBytes": 2048000,
  "builtAt": "2026-07-31T12:00:00Z",
  "workflowRunId": "123456789"
}
```

## Validation

- Reject unknown `schemaVersion` values.
- `repository`, `pullRequestNumber`, `mergeSha`, and `workflowRunId` must match trusted workflow context.
- `mergeSha` must be a full 40-character lowercase hexadecimal commit SHA.
- `artifactName` must be a base file name, not a path, and must identify one x64 NSIS `-setup.exe`.
- Exactly one downloaded file must match `artifactName`.
- Recompute SHA-256 and byte size after download; both must match the manifest.
- Treat `applicationVersion` and `builtAt` as traceability metadata, not authorization inputs.
- Derive release tag from trusted `mergeSha`; never accept an arbitrary tag from the manifest.

## Handoff

- Workflow artifact name: `windows-<full-merge-sha>`
- Contents: one NSIS installer and one `release-manifest.json`
- Retention: the shortest practical period that still permits investigation and retry
- Public release assets: the same installer and manifest after publisher verification
