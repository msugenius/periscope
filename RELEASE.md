# Release a new build

Releases are created when a `dev` -> `master` pull request is merged. Do not
create the Git tag or GitHub release manually.

## 1. Prepare the release commit

Install the local coverage tool once, then start from a clean, current `dev`:

```powershell
cargo install cargo-llvm-cov --locked
git switch dev
git pull --ff-only origin dev
git status --short # Must print nothing.
```

Run the interactive release preparer:

```powershell
npm run release:prepare
```

Choose patch, minor, major, or a custom stable `MAJOR.MINOR.PATCH` version. For
non-interactive use, pass the selection explicitly:

```powershell
npm run release:prepare -- patch
npm run release:prepare -- 1.0.0
```

The script:

1. Requires a clean `dev` branch and consistent current versions.
2. Creates `release/vX.Y.Z`.
3. Updates `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`,
   `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`.
4. Runs release validation, formatting, linting, and coverage-gated tests.
5. Creates `chore(release): vX.Y.Z` as the only release commit.

If a check fails, fix it on the new release branch and rerun the failed checks
before committing. The script never pushes.

## 2. Review and push

```powershell
git show --stat --oneline HEAD
git diff dev...HEAD
$branch = git branch --show-current
git push -u origin $branch
gh pr create --base dev --head $branch --title "chore: release $($branch.Replace('release/', ''))" --body "Prepare the next periScope release."
```

Merge that PR into `dev`. After `dev` is green, open and merge the promotion PR:

```powershell
git switch dev
git pull --ff-only origin dev
$version = (Get-Content package.json -Raw | ConvertFrom-Json).version
gh pr create --base master --head dev --title "release: v$version" --body "Publish periScope v$version."
```

## 3. Verify publication

Merging the promotion PR starts `Windows Release`, which creates tag `vX.Y.Z`
and publishes the installer, signature, `latest.json`, and
`release-manifest.json`.

```powershell
gh run list --workflow release.yml --branch master --limit 1
gh release view "v$version" --json tagName,isDraft,isPrerelease,url,assets
```

The protected GitHub `release` environment must contain
`TAURI_SIGNING_PRIVATE_KEY` and, if encrypted,
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Never store these secrets in the repo.
