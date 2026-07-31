# Contract: Repository Settings

Workflow files expose status checks and publication capability, but repository administrators must connect them to branch and Actions settings.

## `dev` ruleset

- Require a pull request before merge.
- Require `Quality / quality` to pass.
- Require the branch to be current with the target before merge if the repository's merge policy depends on the tested merge result.
- Prevent bypass except for explicitly authorized emergency maintainers.

## `master` ruleset

- Require a pull request before merge.
- Require `Quality / quality` to pass.
- Require the branch to be current with `master` before merge.
- Prevent direct pushes for ordinary contributors.
- Prevent force pushes and deletion.
- Limit bypass to explicitly authorized emergency maintainers; a bypassed/direct commit does not trigger a release.

## Actions permissions

- Set default workflow token permission to read-only.
- Permit the `Windows Release` publisher job to request `contents: write`.
- Do not configure a personal access token for normal publication.
- Allow only reviewed actions; pin workflow references to full commit SHAs.
- Enable Dependabot or an equivalent reviewed update path for pinned action SHAs.

## Release settings

- Enable immutable releases when available so published tags/assets cannot be silently replaced.
- Keep drafts private until installer and manifest verification succeeds.
- Treat the unsigned-installer SmartScreen warning as a documented initial limitation.

## Required verification

After workflow installation:

1. Confirm the check context appears exactly as `Quality / quality`.
2. Confirm failing and missing checks block both protected branches.
3. Confirm the release job receives write permission only after a merged `master` PR and after its read-only build succeeds.
4. Confirm an unmerged close and direct `master` push publish nothing.
