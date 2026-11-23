use std::time::Duration;

use crate::path::last_update_check_file;
use self_update::{cargo_crate_version, check_interval::UpdateCheckGuard};
use tracing::error;

pub struct LatestReleaseInfo {
    pub new_release: Option<NewRelease>,
    // TODO: Move to UI state.
    pub should_show_update: bool,
}

pub struct NewRelease {
    pub tag: String,
    pub release_notes: Option<String>,
}

pub fn check_for_updates() -> LatestReleaseInfo {
    let start = std::time::Instant::now();

    // Only check for updates at most once per day.
    let update_guard = UpdateCheckGuard::new(last_update_check_file(), Duration::from_hours(24));
    let new_release_tag = if update_guard.should_check().unwrap_or_default() {
        // TODO: check if writes fail
        update_guard.record_check().unwrap();

        // TODO: Store Option<Update> and call .update after clicking Update button in update window
        // TODO: set path in archives for binaries.
        // TODO: make sure this resolves to the correct path for each platform
        // TODO macos .app bundle
        // TODO: log all errors
        let update = self_update::backends::github::Update::configure()
            .repo_owner("ScanMountGoat")
            .repo_name("ssbh_editor")
            .current_version(cargo_crate_version!())
            .bin_name("ssbh_editor.exe")
            .build()
            .unwrap();

        update
            .is_update_available()
            .inspect_err(|e| {
                error!("Failed to get latest release info: {e}");
            })
            .ok()
            .flatten()
            .map(|r| r.version().to_string())
    } else {
        None
    };

    let new_release = new_release_tag.map(|new_tag| {
        // Only generate release notes if there is a new update.
        let current_tag = env!("CARGO_PKG_VERSION");
        let release_notes = get_release_notes(current_tag, &new_tag);
        NewRelease {
            tag: new_tag,
            release_notes,
        }
    });

    // TODO: Log instead.
    println!("Check for new release: {:?}", start.elapsed());

    let should_show_update = new_release.is_some();
    LatestReleaseInfo {
        new_release,
        should_show_update,
    }
}

fn get_release_notes(current_tag: &str, new_tag: &str) -> Option<String> {
    let changelog = get_changelog()
        .inspect_err(|e| {
            error!("Failed to download changelog: {e}");
        })
        .ok()?;

    // Find the sections after the current version.
    let start = changelog.find(&format!("## {new_tag}"))?;
    let end = changelog.find(&format!("## {current_tag}"))?;
    changelog.get(start..end).map(|s| s.to_string())
}

fn get_changelog() -> reqwest::Result<String> {
    let url = "https://raw.githubusercontent.com/ScanMountGoat/ssbh_editor/main/CHANGELOG.md";
    reqwest::blocking::get(url)?.text()
}
