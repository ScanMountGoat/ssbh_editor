use std::time::Duration;

use crate::path::last_update_check_file;
use self_update::{
    backends::github::Update, cargo_crate_version, check_interval::UpdateCheckGuard,
};
use tracing::error;

#[derive(Default)]
pub struct LatestReleaseInfo {
    pub new_release: Option<NewRelease>,
    // TODO: Move to UI state.
    pub should_show_update: bool,
}

pub struct NewRelease {
    pub tag: String,
    pub update: Option<Update>,
    pub release_notes: Option<String>,
}

pub fn check_for_updates() -> anyhow::Result<LatestReleaseInfo> {
    let start = std::time::Instant::now();

    // Only check for updates at most once per day.
    let update_guard = UpdateCheckGuard::new(last_update_check_file(), Duration::from_hours(24));
    let (update, new_release_tag) = if update_guard.should_check()? {
        update_guard.record_check()?;

        // TODO: set path in archives for binaries.
        // TODO: make sure this resolves to the correct path for each platform
        // TODO macos .app bundle

        // TODO: configure this and test on platforms other than windows
        let update = configure_update()?;

        let tag = update
            .is_update_available()?
            .map(|r| r.version().to_string());
        (Some(update), tag)
    } else {
        (None, None)
    };

    let new_release = new_release_tag.map(|new_tag| {
        // Only generate release notes if there is a new update.
        let current_tag = env!("CARGO_PKG_VERSION");
        let release_notes = get_release_notes(current_tag, &new_tag);
        NewRelease {
            tag: new_tag,
            update,
            release_notes,
        }
    });

    // TODO: Log instead.
    println!("Check for new release: {:?}", start.elapsed());

    let should_show_update = new_release.is_some();
    Ok(LatestReleaseInfo {
        new_release,
        should_show_update,
    })
}

fn configure_update() -> Result<Update, anyhow::Error> {
    let target = if cfg!(target_os = "windows") {
        "win_x64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "macos_apple_silicon"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "macos_intel"
    } else if cfg!(target_os = "linux") {
        "linux_x64"
    } else {
        anyhow::bail!(
            "Updates for {} {} are not supported",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    };
    let bin_name = if cfg!(target_os = "windows") {
        "ssbh_editor.exe"
    } else {
        "ssbh_editor"
    };

    let mut update = self_update::backends::github::Update::configure();
    update
        .repo_owner("ScanMountGoat")
        .repo_name("ssbh_editor")
        .current_version(cargo_crate_version!())
        .no_confirm(true)
        .show_output(false)
        .target(target)
        .bin_name(bin_name);

    #[cfg(target_os = "macos")]
    {
        update.bundle_path_in_archive("ssbh_editor.app");
    }

    update.build().map_err(Into::into)
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
