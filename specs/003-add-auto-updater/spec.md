# Feature Specification: Automatic Application Updates

**Feature Branch**: `[003-add-auto-updater]`

**Created**: 2026-07-31

**Status**: Draft

**Input**: User description: "Add autoupdater, it should check https://github.com/msugenius/periscope/releases for latest release and compare version i guess. Also releases built by main branch CD pipeline of course should have semver versioning"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Learn that an update is available (Priority: P1)

As a user, I am notified when a newer stable periScope release is available, without having to visit the releases page or interrupt my normal use of the application.

**Why this priority**: Detecting and clearly presenting a newer release is the minimum useful updater capability and gives users a reliable path to supported versions.

**Independent Test**: Launch an older installed version while a newer compatible stable release exists, then verify the application remains usable and presents the newer version and release summary once.

**Acceptance Scenarios**:

1. **Given** the installed version is older than the latest compatible stable release, **When** the application performs its launch-time update check, **Then** the user is offered that release with its version and release summary.
2. **Given** the installed version equals or exceeds the latest compatible stable release, **When** the check completes, **Then** no update prompt is shown.
3. **Given** the release service is unreachable or returns unusable release information, **When** the check fails, **Then** the application continues operating normally and does not claim that an update is available.
4. **Given** the latest publication is a draft, prerelease, invalidly versioned, or incompatible with the installed application, **When** releases are evaluated, **Then** it is not offered as a stable update.

---

### User Story 2 - Install an available update safely (Priority: P2)

As a user, I can approve an offered update and have periScope install the correct Windows release with clear progress and failure feedback.

**Why this priority**: Detection alone still leaves users to update manually; safe installation completes the user value while preserving user control.

**Independent Test**: Offer a valid newer release to an older installation, approve it, and verify that the application installs and restarts as the offered version while preserving existing settings.

**Acceptance Scenarios**:

1. **Given** a valid update is offered, **When** the user approves installation, **Then** the matching update package is obtained, verified, installed, and the user is prompted or guided to restart when required.
2. **Given** an update is offered, **When** the user declines or dismisses it, **Then** the current version remains unchanged and usable for the rest of that session.
3. **Given** obtaining, verifying, or installing the update fails, **When** the attempt ends, **Then** the current installation remains runnable and the user receives an actionable, non-technical failure message.
4. **Given** the application restarts after a successful update, **When** its version is checked, **Then** it matches the version the user approved and existing user settings remain available.

---

### User Story 3 - Publish updater-ready semantic releases (Priority: P3)

As a maintainer, I can merge a release-ready change into the primary release branch and have CD publish one Windows release whose version, metadata, and artifacts can be consumed safely by installed applications.

**Why this priority**: Automatic checking and installation depend on a trustworthy, monotonically versioned release source, but this delivery work can be verified independently of the application experience.

**Independent Test**: Merge a change containing a valid version increase into the primary release branch, then verify that one stable release appears at the project releases page with matching semantic versions, update information, and Windows artifacts.

**Acceptance Scenarios**:

1. **Given** a release-ready merge declares a version greater than the latest stable release, **When** CD succeeds on the primary release branch, **Then** exactly one stable release is published with that semantic version.
2. **Given** a release-ready merge has inconsistent, invalid, reused, or lower version declarations, **When** CD validates the release, **Then** publication fails before any stable release is created.
3. **Given** a release build succeeds, **When** it is published, **Then** the release identifier, application version, update information, and Windows artifacts all identify the same version and source commit.
4. **Given** release publication is incomplete or an artifact cannot be verified, **When** CD finishes, **Then** no update is advertised to installed applications as ready.

### Edge Cases

- The releases source contains multiple stable releases whose publication dates and semantic versions are ordered differently; the greatest compatible semantic version wins.
- The installed version is newer than every public stable release, as may occur for a development build; no downgrade is offered.
- The latest release uses a valid prerelease suffix; it is ignored by the stable update channel.
- Release information is missing a version, expected Windows package, integrity proof, release summary, or compatible platform declaration; that release is not offered.
- The releases source times out, is rate-limited, redirects, returns malformed information, or is temporarily unavailable; startup and normal application use continue.
- The network disconnects, storage becomes insufficient, or permission is denied during download or installation; the current version remains usable and retry is possible later.
- A newer release is published while a check or download is already in progress; the current attempt remains pinned to the version the user approved.
- More than one application instance checks or attempts installation; duplicate prompts and concurrent modification of the installation are prevented.
- A release is withdrawn after it is detected but before installation; verification fails safely and the withdrawn package is not installed.
- The user dismisses an update repeatedly; the application checks again on a later launch but does not repeatedly prompt during the same session.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST check the periScope releases source at `https://github.com/msugenius/periscope/releases` once per application launch after the application is usable.
- **FR-002**: The update check MUST NOT block startup, crosshair operation, settings access, or application shutdown.
- **FR-003**: The application MUST identify the greatest compatible, published, non-draft, stable release with a valid semantic version.
- **FR-004**: The application MUST compare the installed version and candidate release using Semantic Versioning precedence rules rather than publication time or text sorting.
- **FR-005**: The application MUST offer an update only when the candidate release version is strictly greater than the installed version.
- **FR-006**: The update offer MUST identify the installed version, offered version, release summary, and the choices to install or dismiss.
- **FR-007**: The application MUST require user approval before downloading or installing an update.
- **FR-008**: Dismissing or declining an update MUST leave the installed version unchanged and suppress further update prompts for that application session.
- **FR-009**: An approved update MUST remain pinned to the exact version, release identity, source commit, and compatible Windows package presented to the user.
- **FR-010**: Before installation, the application MUST verify that the downloaded package is authentic, intact, compatible, and matches the approved release.
- **FR-011**: The application MUST reject and MUST NOT install an update that is untrusted, corrupted, incomplete, incompatible, withdrawn, or different from the approved release.
- **FR-012**: The updater MUST report meaningful progress while obtaining and applying an approved update, including any required restart action.
- **FR-013**: A failed or cancelled update attempt MUST preserve a runnable current installation and MUST provide a clear recovery or retry path.
- **FR-014**: A successful update MUST preserve user settings and MUST result in an installed version equal to the version the user approved.
- **FR-015**: The application MUST NOT offer automatic downgrades or prerelease updates through the stable update channel.
- **FR-016**: Update-check failures MUST NOT prevent normal application use and MUST NOT produce a false update-available notification.
- **FR-017**: The system MUST prevent duplicate prompts in one session and concurrent attempts to modify the same installation.
- **FR-018**: Stable Windows releases MUST be produced only by successful CD processing of the repository's primary release branch, currently `master` unless that branch is renamed.
- **FR-019**: Every stable release MUST use a unique, valid `MAJOR.MINOR.PATCH` semantic version greater than the preceding stable release.
- **FR-020**: The intended release version MUST be declared in the release-ready source before merge and MUST be validated before publication.
- **FR-021**: All authoritative application version declarations for a release MUST agree before the Windows build is published.
- **FR-022**: The published release identifier, displayed release version, packaged application version, update information, artifact names, and source commit MUST be mutually traceable and refer to the same release.
- **FR-023**: Every published stable release MUST include the compatible Windows update package, authenticity and integrity proof, release summary, publication time, and information required to locate the package.
- **FR-024**: CD MUST publish update information only after the corresponding Windows package has been built and verified successfully.
- **FR-025**: Failed, cancelled, duplicate, or partial CD runs MUST NOT expose a release as an installable stable update.
- **FR-026**: Retrying CD for the same version and source commit MUST be idempotent and MUST NOT create duplicate stable releases.
- **FR-027**: Secrets used to sign or publish updates MUST be limited to the release process, MUST NOT be available to untrusted change validation, and MUST NOT appear in logs or artifacts.

### Test Requirements *(mandatory)*

- **TR-001**: Automated tests MUST cover semantic-version precedence, including major, minor, patch, equal, lower, malformed, build-metadata, and prerelease cases.
- **TR-002**: Automated tests MUST cover update detection with no release, an eligible stable release, drafts, prereleases, incompatible packages, missing fields, malformed responses, timeouts, rate limits, and offline operation.
- **TR-003**: Automated tests MUST cover user approval, dismissal, progress, restart guidance, success, cancellation, download failure, integrity failure, permission failure, insufficient storage, and preservation of the prior runnable installation.
- **TR-004**: Controlled integration tests MUST verify consistency among the declared version, release identity, update information, Windows package, installed version, and source commit.
- **TR-005**: Release-pipeline tests MUST verify valid version increases, rejection of invalid or non-increasing versions, retry idempotency, prevention of partial publication, and protection of signing and publishing secrets.
- **TR-006**: Automated tests MUST maintain at least 80% line coverage for each instrumented production codebase; updater platform-boundary behavior may be excluded only when the exclusion and equivalent controlled verification are documented.

### Performance and Footprint Requirements *(mandatory)*

- **PF-001**: Update checking MUST begin only after the application is usable and MUST add no more than 100 milliseconds to measured interactive startup time at the 95th percentile.
- **PF-002**: Under normal network and service conditions, 95% of update checks MUST produce an update offer or a no-update result within 10 seconds.
- **PF-003**: The updater MUST perform at most one automatic network check per application launch and MUST perform no periodic polling, continuous redraw, or ongoing network activity after the check completes.
- **PF-004**: After an update check or attempt completes, updater-related idle CPU usage MUST be indistinguishable from the pre-feature baseline and updater-specific memory MUST be released except for the minimal current-session status.
- **PF-005**: Updater capability MUST add no more than 2 MiB to the installed application footprint and MUST avoid dependencies that duplicate existing application or platform capabilities without a measured need.
- **PF-006**: Downloaded update data MUST be limited to the selected compatible package and small release metadata, and incomplete temporary data MUST be removed after failure or cancellation.

### Key Entities

- **Installed Version**: The semantic version and platform identity of the currently running application.
- **Release Candidate**: A published release considered for update, including its semantic version, stability state, compatibility, summary, source commit, and package references.
- **Update Package**: The version-pinned Windows distributable and its authenticity and integrity information.
- **Update Attempt**: The current session's detection, user decision, progress, terminal result, and failure details for one pinned release.
- **Release Version Declaration**: The authoritative semantic version selected before merge and required to agree across source, build, release, and update information.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of semantic-version test cases, the application offers only the greatest compatible stable release whose version is strictly greater than the installed version.
- **SC-002**: Under normal connectivity, at least 95% of launch-time checks reach an update or no-update result within 10 seconds without delaying interactive startup by more than 100 milliseconds at the 95th percentile.
- **SC-003**: In 100% of offline, unavailable, malformed, untrusted, and incompatible-release tests, the application remains usable and no invalid update is offered or installed.
- **SC-004**: At least 90% of usability-test participants can identify the offered version and start or dismiss an update on their first attempt without external instructions.
- **SC-005**: In 100% of successful update tests, the restarted application matches the approved version and retains all existing user settings.
- **SC-006**: In 100% of failed or cancelled update tests, the previously installed version remains runnable and the user receives a recovery or retry path.
- **SC-007**: In 100% of release-pipeline tests, invalid, inconsistent, reused, lower, partial, or unverified releases are prevented from becoming installable stable updates.
- **SC-008**: A maintainer can trace any published stable version to its source commit, release summary, update information, and exact Windows package in under 2 minutes.
- **SC-009**: Measurements confirm no periodic update polling, no post-check idle CPU regression, no more than 2 MiB added installed footprint, and no more than 100 milliseconds added interactive startup time at the 95th percentile.
- **SC-010**: Automated test results maintain at least 80% line coverage for each instrumented production codebase.

## Assumptions

- This feature extends the Windows build and publication capability specified in `specs/002-cicd-release-pipeline`; it does not replace the existing quality gates, changelog, or release traceability requirements.
- The repository's current primary release branch is `master`; “main branch” means this role and continues to apply if the branch is later renamed.
- The first version of the updater serves the stable Windows channel only; prerelease channels, other operating systems, portable-package replacement, and store-managed updates are outside scope.
- A maintainer selects and declares the next semantic version in the release-ready change. Automatic inference of major, minor, or patch increments from commit text is outside scope.
- Stable release identifiers use the conventional `vMAJOR.MINOR.PATCH` form, while semantic comparison uses the numeric version itself.
- Update checks occur once after each launch rather than on a timer. A manual “check now” action and configurable schedules are outside the smallest useful scope.
- Updates require explicit user approval; silent background installation and forced updates are outside scope.
- Existing user settings remain compatible across ordinary updates. Any future version requiring settings migration must define that migration in its own feature specification.
- The public project release service remains the authoritative discovery source and can publish the metadata, artifacts, and integrity information needed by installed applications.
