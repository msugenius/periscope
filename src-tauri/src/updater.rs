use serde::Serialize;
use std::{
    cmp::Ordering,
    sync::{Mutex, MutexGuard},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

pub const UPDATE_EVENT: &str = "periscope://updater-state";
const PLATFORM: &str = "windows-x86_64";

#[derive(Clone, Debug, PartialEq, Eq)]
struct StableVersion {
    text: String,
    parts: [u64; 3],
}

impl StableVersion {
    fn parse(value: &str) -> Result<Self, MetadataError> {
        let identifiers: Vec<_> = value.split('.').collect();
        if identifiers.len() != 3 {
            return Err(MetadataError);
        }
        let mut parts = [0; 3];
        for (index, identifier) in identifiers.into_iter().enumerate() {
            if identifier.is_empty()
                || !identifier.bytes().all(|byte| byte.is_ascii_digit())
                || (identifier.len() > 1 && identifier.starts_with('0'))
            {
                return Err(MetadataError);
            }
            parts[index] = identifier.parse().map_err(|_| MetadataError)?;
        }
        Ok(Self {
            text: value.to_owned(),
            parts,
        })
    }
}

impl Ord for StableVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.parts.cmp(&other.parts)
    }
}

impl PartialOrd for StableVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstalledVersion(StableVersion);

impl InstalledVersion {
    pub fn new(version: &str) -> Result<Self, MetadataError> {
        StableVersion::parse(version).map(Self)
    }
}

#[derive(Clone, Debug)]
pub struct CandidateMetadata {
    pub version: String,
    pub notes: Option<String>,
    pub source_commit: Option<String>,
    pub platform: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseCandidate {
    pub version: String,
    pub notes: String,
    pub source_commit: String,
    pub platform: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdatePhase {
    Idle,
    Checking,
    UpToDate,
    Available,
    Dismissed,
    Downloading,
    Installing,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSession {
    pub phase: UpdatePhase,
    pub installed_version: String,
    pub candidate: Option<ReleaseCandidate>,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub failure_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckFailure {
    Offline,
    Timeout,
    RateLimited,
    InvalidMetadata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionError {
    NotAvailable,
    VersionMismatch,
}

impl ActionError {
    fn safe_message(self) -> &'static str {
        match self {
            Self::NotAvailable => "update-not-available",
            Self::VersionMismatch => "update-version-mismatch",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallFailure {
    Download,
    Signature,
    Install,
}

#[derive(Clone, Copy, Debug)]
pub struct MetadataError;

impl ReleaseCandidate {
    pub fn try_from_metadata(
        installed: &InstalledVersion,
        metadata: CandidateMetadata,
    ) -> Result<Option<Self>, MetadataError> {
        let version = StableVersion::parse(&metadata.version)?;
        if metadata.platform != PLATFORM {
            return Err(MetadataError);
        }
        let notes = metadata
            .notes
            .filter(|notes| !notes.trim().is_empty())
            .ok_or(MetadataError)?;
        let source_commit = metadata
            .source_commit
            .filter(|commit| {
                commit.len() == 40
                    && commit
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            .ok_or(MetadataError)?;
        if version <= installed.0 {
            return Ok(None);
        }
        Ok(Some(Self {
            version: version.text,
            notes,
            source_commit,
            platform: metadata.platform,
        }))
    }
}

#[derive(Debug)]
pub struct SessionCore {
    snapshot: UpdateSession,
}

impl SessionCore {
    pub fn new(installed_version: &str) -> Self {
        let installed = InstalledVersion::new(installed_version)
            .expect("the packaged application version must be stable SemVer");
        Self {
            snapshot: UpdateSession {
                phase: UpdatePhase::Idle,
                installed_version: installed.0.text,
                candidate: None,
                downloaded_bytes: None,
                total_bytes: None,
                failure_code: None,
                message: None,
            },
        }
    }

    pub fn snapshot(&self) -> UpdateSession {
        self.snapshot.clone()
    }

    pub fn begin_check(&mut self) -> bool {
        if self.snapshot.phase != UpdatePhase::Idle {
            return false;
        }
        self.snapshot.phase = UpdatePhase::Checking;
        true
    }

    pub fn complete_check(&mut self, candidate: Option<ReleaseCandidate>) {
        self.snapshot.phase = if candidate.is_some() {
            UpdatePhase::Available
        } else {
            UpdatePhase::UpToDate
        };
        self.snapshot.candidate = candidate;
        self.snapshot.failure_code = None;
        self.snapshot.message = None;
    }

    pub fn fail(&mut self, failure: CheckFailure) {
        let (code, message) = match failure {
            CheckFailure::Offline => (
                "offline",
                "Could not check for updates while offline. Try again next time.",
            ),
            CheckFailure::Timeout => (
                "timeout",
                "The update check timed out. Try again next time.",
            ),
            CheckFailure::RateLimited => (
                "rate-limited",
                "Update checks are temporarily limited. Try again next time.",
            ),
            CheckFailure::InvalidMetadata => (
                "invalid-metadata",
                "The update information was not valid. Try again next time.",
            ),
        };
        self.snapshot.phase = UpdatePhase::Failed;
        self.snapshot.candidate = None;
        self.snapshot.downloaded_bytes = None;
        self.snapshot.total_bytes = None;
        self.snapshot.failure_code = Some(code.into());
        self.snapshot.message = Some(message.into());
    }

    fn matching_available_candidate(&self, version: &str) -> Result<(), ActionError> {
        if self.snapshot.phase != UpdatePhase::Available {
            return Err(ActionError::NotAvailable);
        }
        if self
            .snapshot
            .candidate
            .as_ref()
            .map(|candidate| candidate.version.as_str())
            != Some(version)
        {
            return Err(ActionError::VersionMismatch);
        }
        Ok(())
    }

    pub fn dismiss(&mut self, version: &str) -> Result<(), ActionError> {
        self.matching_available_candidate(version)?;
        self.snapshot.phase = UpdatePhase::Dismissed;
        self.snapshot.candidate = None;
        Ok(())
    }

    pub fn begin_install(&mut self, version: &str) -> Result<(), ActionError> {
        self.matching_available_candidate(version)?;
        self.snapshot.phase = UpdatePhase::Downloading;
        self.snapshot.downloaded_bytes = Some(0);
        self.snapshot.total_bytes = None;
        Ok(())
    }

    pub fn record_progress(&mut self, chunk_bytes: u64, total_bytes: Option<u64>) {
        if self.snapshot.phase != UpdatePhase::Downloading {
            return;
        }
        let downloaded = self
            .snapshot
            .downloaded_bytes
            .unwrap_or(0)
            .saturating_add(chunk_bytes);
        let total = total_bytes
            .filter(|total| *total > 0)
            .or(self.snapshot.total_bytes);
        self.snapshot.downloaded_bytes =
            Some(total.map_or(downloaded, |total| downloaded.min(total)));
        self.snapshot.total_bytes = total;
    }

    pub fn finish_download(&mut self) {
        if self.snapshot.phase == UpdatePhase::Downloading {
            self.snapshot.phase = UpdatePhase::Installing;
        }
    }

    pub fn fail_install(&mut self, failure: InstallFailure) {
        let (code, message) = match failure {
            InstallFailure::Download => (
                "download-failed",
                "The update could not be downloaded. Restart periScope to try again.",
            ),
            InstallFailure::Signature => (
                "verification-failed",
                "The update could not be verified. Your current installation is unchanged. Restart periScope to try again.",
            ),
            InstallFailure::Install => (
                "install-failed",
                "The update could not be installed. Your current installation is still available. Restart periScope to try again.",
            ),
        };
        self.snapshot.phase = UpdatePhase::Failed;
        self.snapshot.candidate = None;
        self.snapshot.failure_code = Some(code.into());
        self.snapshot.message = Some(message.into());
    }
}

struct UpdaterStore {
    core: SessionCore,
    pending: Option<Update>,
}

pub struct UpdaterState(Mutex<UpdaterStore>);

impl UpdaterState {
    pub fn new(installed_version: &str) -> Self {
        Self(Mutex::new(UpdaterStore {
            core: SessionCore::new(installed_version),
            pending: None,
        }))
    }

    fn lock(&self) -> MutexGuard<'_, UpdaterStore> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn snapshot(&self) -> UpdateSession {
        self.lock().core.snapshot()
    }
}

fn emit_snapshot(app: &AppHandle, snapshot: &UpdateSession) {
    let _ = app.emit(UPDATE_EVENT, snapshot);
}

fn classify_check_error(error: &str) -> CheckFailure {
    let lower = error.to_ascii_lowercase();
    if lower.contains("429") || lower.contains("rate limit") {
        CheckFailure::RateLimited
    } else if lower.contains("timed out") || lower.contains("timeout") {
        CheckFailure::Timeout
    } else {
        CheckFailure::Offline
    }
}

fn metadata_from_update(update: &Update) -> CandidateMetadata {
    CandidateMetadata {
        version: update.version.clone(),
        notes: update.body.clone(),
        source_commit: update
            .raw_json
            .get("sourceCommit")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        platform: update.target.clone(),
    }
}

async fn check_for_update(app: AppHandle) {
    let result = match app
        .updater_builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(updater) => updater.check().await.map_err(|error| error.to_string()),
        Err(error) => Err(error.to_string()),
    };

    let state = app.state::<UpdaterState>();
    let snapshot = {
        let mut store = state.lock();
        match result {
            Ok(Some(update)) => {
                let installed = InstalledVersion::new(&store.core.snapshot().installed_version)
                    .expect("managed installed version was validated at startup");
                match ReleaseCandidate::try_from_metadata(&installed, metadata_from_update(&update))
                {
                    Ok(candidate) => {
                        store.core.complete_check(candidate.clone());
                        store.pending = candidate.map(|_| update);
                    }
                    Err(_) => {
                        store.pending = None;
                        store.core.fail(CheckFailure::InvalidMetadata);
                    }
                }
            }
            Ok(None) => {
                store.pending = None;
                store.core.complete_check(None);
            }
            Err(error) => {
                store.pending = None;
                store.core.fail(classify_check_error(&error));
            }
        }
        store.core.snapshot()
    };
    emit_snapshot(&app, &snapshot);
}

#[tauri::command]
pub fn get_update_status(state: State<'_, UpdaterState>) -> UpdateSession {
    state.snapshot()
}

#[tauri::command]
pub fn start_update_check(app: AppHandle, state: State<'_, UpdaterState>) -> UpdateSession {
    let (accepted, snapshot) = {
        let mut store = state.lock();
        let accepted = store.core.begin_check();
        (accepted, store.core.snapshot())
    };
    if accepted {
        emit_snapshot(&app, &snapshot);
        tauri::async_runtime::spawn(check_for_update(app));
    }
    snapshot
}

#[tauri::command]
pub fn dismiss_update(
    version: String,
    app: AppHandle,
    state: State<'_, UpdaterState>,
) -> Result<UpdateSession, String> {
    let snapshot = {
        let mut store = state.lock();
        store
            .core
            .dismiss(&version)
            .map_err(|error| error.safe_message())?;
        store.pending = None;
        store.core.snapshot()
    };
    emit_snapshot(&app, &snapshot);
    Ok(snapshot)
}

fn classify_download_error(error: &str) -> InstallFailure {
    if error.to_ascii_lowercase().contains("signature") {
        InstallFailure::Signature
    } else {
        InstallFailure::Download
    }
}

fn fail_install(app: &AppHandle, failure: InstallFailure) -> UpdateSession {
    let state = app.state::<UpdaterState>();
    let snapshot = {
        let mut store = state.lock();
        store.pending = None;
        store.core.fail_install(failure);
        store.core.snapshot()
    };
    emit_snapshot(app, &snapshot);
    snapshot
}

#[tauri::command]
pub async fn install_update(
    version: String,
    app: AppHandle,
    state: State<'_, UpdaterState>,
) -> Result<UpdateSession, String> {
    let (update, downloading) = {
        let mut store = state.lock();
        if store.pending.is_none() {
            return Err("update-not-available".into());
        }
        store
            .core
            .begin_install(&version)
            .map_err(|error| error.safe_message())?;
        let update = store
            .pending
            .take()
            .expect("pending update was checked before transition");
        (update, store.core.snapshot())
    };
    emit_snapshot(&app, &downloading);

    let progress_app = app.clone();
    let download = update
        .download(
            move |chunk_bytes, total_bytes| {
                let state = progress_app.state::<UpdaterState>();
                let snapshot = {
                    let mut store = state.lock();
                    store.core.record_progress(chunk_bytes as u64, total_bytes);
                    store.core.snapshot()
                };
                emit_snapshot(&progress_app, &snapshot);
            },
            || {},
        )
        .await;
    let bytes = match download {
        Ok(bytes) => bytes,
        Err(error) => {
            return Ok(fail_install(
                &app,
                classify_download_error(&error.to_string()),
            ));
        }
    };

    let installing = {
        let state = app.state::<UpdaterState>();
        let mut store = state.lock();
        store.core.finish_download();
        store.core.snapshot()
    };
    emit_snapshot(&app, &installing);

    if update.install(bytes).is_err() {
        return Ok(fail_install(&app, InstallFailure::Install));
    }
    Ok(installing)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn metadata(version: &str) -> CandidateMetadata {
        CandidateMetadata {
            version: version.into(),
            notes: Some("A safer release".into()),
            source_commit: Some(SHA.into()),
            platform: "windows-x86_64".into(),
        }
    }

    #[test]
    fn check_starts_only_once_for_the_process() {
        let mut session = SessionCore::new("0.1.0");
        assert!(session.begin_check());
        assert_eq!(session.snapshot().phase, UpdatePhase::Checking);
        assert!(!session.begin_check());
        session.fail(CheckFailure::Offline);
        assert!(!session.begin_check());
    }

    #[test]
    fn accepts_only_a_newer_stable_compatible_release() {
        let installed = InstalledVersion::new("1.2.3").unwrap();
        let candidate = ReleaseCandidate::try_from_metadata(&installed, metadata("2.0.0"))
            .unwrap()
            .unwrap();
        assert_eq!(candidate.version, "2.0.0");
        assert_eq!(candidate.source_commit, SHA);
    }

    #[test]
    fn equal_and_lower_versions_are_up_to_date() {
        let installed = InstalledVersion::new("1.2.3").unwrap();
        assert!(
            ReleaseCandidate::try_from_metadata(&installed, metadata("1.2.3"))
                .unwrap()
                .is_none()
        );
        assert!(
            ReleaseCandidate::try_from_metadata(&installed, metadata("1.2.2"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn rejects_prerelease_build_leading_zero_and_malformed_versions() {
        let installed = InstalledVersion::new("1.2.3").unwrap();
        for version in ["1.3.0-beta.1", "1.3.0+build", "01.3.0", "1.3", "v1.3.0"] {
            assert!(ReleaseCandidate::try_from_metadata(&installed, metadata(version)).is_err());
        }
    }

    #[test]
    fn rejects_incompatible_or_incomplete_metadata() {
        let installed = InstalledVersion::new("1.2.3").unwrap();
        let mut wrong_platform = metadata("1.3.0");
        wrong_platform.platform = "linux-x86_64".into();
        assert!(ReleaseCandidate::try_from_metadata(&installed, wrong_platform).is_err());

        let mut missing_notes = metadata("1.3.0");
        missing_notes.notes = None;
        assert!(ReleaseCandidate::try_from_metadata(&installed, missing_notes).is_err());

        let mut bad_commit = metadata("1.3.0");
        bad_commit.source_commit = Some("secret/raw/server/body".into());
        assert!(ReleaseCandidate::try_from_metadata(&installed, bad_commit).is_err());
    }

    #[test]
    fn maps_operational_failures_to_safe_stable_messages() {
        for (failure, code) in [
            (CheckFailure::Offline, "offline"),
            (CheckFailure::Timeout, "timeout"),
            (CheckFailure::RateLimited, "rate-limited"),
            (CheckFailure::InvalidMetadata, "invalid-metadata"),
        ] {
            let mut session = SessionCore::new("0.1.0");
            session.begin_check();
            session.fail(failure);
            let snapshot = session.snapshot();
            assert_eq!(snapshot.phase, UpdatePhase::Failed);
            assert_eq!(snapshot.failure_code.as_deref(), Some(code));
            assert!(!snapshot.message.as_deref().unwrap().contains("http"));
            assert!(snapshot.candidate.is_none());
        }
    }

    #[test]
    fn no_update_and_candidate_transitions_are_complete_snapshots() {
        let mut session = SessionCore::new("0.1.0");
        session.begin_check();
        session.complete_check(None);
        assert_eq!(session.snapshot().phase, UpdatePhase::UpToDate);

        let mut session = SessionCore::new("0.1.0");
        session.begin_check();
        let installed = InstalledVersion::new("0.1.0").unwrap();
        let candidate = ReleaseCandidate::try_from_metadata(&installed, metadata("0.2.0"))
            .unwrap()
            .unwrap();
        session.complete_check(Some(candidate));
        assert_eq!(session.snapshot().phase, UpdatePhase::Available);
        assert_eq!(session.snapshot().candidate.unwrap().version, "0.2.0");
    }

    fn available_session() -> SessionCore {
        let mut session = SessionCore::new("0.1.0");
        session.begin_check();
        let installed = InstalledVersion::new("0.1.0").unwrap();
        let candidate = ReleaseCandidate::try_from_metadata(&installed, metadata("0.2.0"))
            .unwrap()
            .unwrap();
        session.complete_check(Some(candidate));
        session
    }

    #[test]
    fn pins_actions_to_the_exact_offered_version() {
        let mut session = available_session();
        assert_eq!(
            session.begin_install("0.3.0"),
            Err(ActionError::VersionMismatch)
        );
        assert_eq!(session.snapshot().phase, UpdatePhase::Available);
        assert_eq!(session.begin_install("0.2.0"), Ok(()));
        assert_eq!(session.snapshot().phase, UpdatePhase::Downloading);
        assert_eq!(
            session.begin_install("0.2.0"),
            Err(ActionError::NotAvailable)
        );
    }

    #[test]
    fn dismissal_releases_the_candidate_and_is_not_repeatable() {
        let mut session = available_session();
        assert_eq!(session.dismiss("0.2.0"), Ok(()));
        assert_eq!(session.snapshot().phase, UpdatePhase::Dismissed);
        assert!(session.snapshot().candidate.is_none());
        assert_eq!(session.dismiss("0.2.0"), Err(ActionError::NotAvailable));
    }

    #[test]
    fn progress_is_monotonic_and_never_exceeds_the_total() {
        let mut session = available_session();
        session.begin_install("0.2.0").unwrap();
        session.record_progress(40, Some(100));
        session.record_progress(10, Some(100));
        assert_eq!(session.snapshot().downloaded_bytes, Some(50));
        assert_eq!(session.snapshot().total_bytes, Some(100));
        session.record_progress(80, Some(100));
        assert_eq!(session.snapshot().downloaded_bytes, Some(100));
        session.finish_download();
        assert_eq!(session.snapshot().phase, UpdatePhase::Installing);
    }

    #[test]
    fn install_failures_are_safe_and_release_pending_identity() {
        for failure in [
            InstallFailure::Download,
            InstallFailure::Signature,
            InstallFailure::Install,
        ] {
            let mut session = available_session();
            session.begin_install("0.2.0").unwrap();
            session.fail_install(failure);
            let snapshot = session.snapshot();
            assert_eq!(snapshot.phase, UpdatePhase::Failed);
            assert!(snapshot.candidate.is_none());
            assert!(snapshot.message.unwrap().contains("Restart periScope"));
            assert_eq!(
                session.begin_install("0.2.0"),
                Err(ActionError::NotAvailable)
            );
        }
    }

    #[test]
    fn helper_classification_is_stable_and_does_not_echo_diagnostics() {
        assert_eq!(
            ActionError::NotAvailable.safe_message(),
            "update-not-available"
        );
        assert_eq!(
            ActionError::VersionMismatch.safe_message(),
            "update-version-mismatch"
        );
        assert_eq!(
            classify_check_error("HTTP 429 from endpoint"),
            CheckFailure::RateLimited
        );
        assert_eq!(
            classify_check_error("rate limit exceeded"),
            CheckFailure::RateLimited
        );
        assert_eq!(
            classify_check_error("request timed out"),
            CheckFailure::Timeout
        );
        assert_eq!(classify_check_error("TIMEOUT"), CheckFailure::Timeout);
        assert_eq!(classify_check_error("dns failed"), CheckFailure::Offline);
        assert_eq!(
            classify_download_error("Signature verification failed"),
            InstallFailure::Signature
        );
        assert_eq!(
            classify_download_error("connection closed"),
            InstallFailure::Download
        );
    }

    #[test]
    fn managed_state_starts_idle_and_recovers_a_poisoned_lock() {
        use std::sync::Arc;

        let state = Arc::new(UpdaterState::new("0.1.0"));
        assert_eq!(state.snapshot().phase, UpdatePhase::Idle);
        let poison = Arc::clone(&state);
        assert!(
            std::thread::spawn(move || {
                let _guard = poison.0.lock().unwrap();
                panic!("poison updater state for recovery coverage");
            })
            .join()
            .is_err()
        );
        assert_eq!(state.snapshot().installed_version, "0.1.0");
    }

    #[test]
    fn progress_ignores_other_phases_and_supports_unknown_lengths() {
        let mut session = SessionCore::new("0.1.0");
        session.record_progress(20, Some(100));
        session.finish_download();
        assert_eq!(session.snapshot().phase, UpdatePhase::Idle);
        assert_eq!(session.snapshot().downloaded_bytes, None);

        session.begin_check();
        let installed = InstalledVersion::new("0.1.0").unwrap();
        let candidate = ReleaseCandidate::try_from_metadata(&installed, metadata("0.2.0"))
            .unwrap()
            .unwrap();
        session.complete_check(Some(candidate));
        session.begin_install("0.2.0").unwrap();
        session.record_progress(12, None);
        session.record_progress(3, Some(0));
        assert_eq!(session.snapshot().downloaded_bytes, Some(15));
        assert_eq!(session.snapshot().total_bytes, None);
    }

    #[test]
    fn rejects_empty_notes_uppercase_commits_and_overflowing_versions() {
        let installed = InstalledVersion::new("1.2.3").unwrap();
        let mut empty_notes = metadata("1.3.0");
        empty_notes.notes = Some("   ".into());
        assert!(ReleaseCandidate::try_from_metadata(&installed, empty_notes).is_err());

        let mut uppercase_commit = metadata("1.3.0");
        uppercase_commit.source_commit = Some(SHA.to_uppercase());
        assert!(ReleaseCandidate::try_from_metadata(&installed, uppercase_commit).is_err());
        assert!(InstalledVersion::new("18446744073709551616.0.0").is_err());
    }
}
