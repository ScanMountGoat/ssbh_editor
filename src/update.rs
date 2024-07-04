use crate::path::last_update_check_file;
use chrono::{DateTime, Utc};
use octocrab::models::repos::Release;

pub struct LatestReleaseInfo {
    pub update_check_time: DateTime<Utc>,
    pub new_release_tag: Option<String>,
    pub release_notes: Option<String>,
    pub should_show_update: bool,
}

pub fn check_for_updates() -> LatestReleaseInfo {
    let last_update_check_file = last_update_check_file();

    let previous_update_check_time: Option<DateTime<Utc>> =
        std::fs::read_to_string(last_update_check_file)
            .unwrap_or_default()
            .parse()
            .ok();

    let update_check_time = Utc::now();

    // Only check for updates at most once per day.
    // TODO: Add logging for update check?
    let start = std::time::Instant::now();
    let should_check_for_update =
        should_check_for_release(previous_update_check_time, update_check_time);
    let new_release_tag = if should_check_for_update {
        get_latest_release().map(|r| r.tag_name)
    } else {
        None
    };

    // Check if the latest github release is more recent than the current one.
    let current_tag = env!("CARGO_PKG_VERSION");
    let should_show_update = if let Some(new_release_tag) = &new_release_tag {
        new_release_tag.as_str() > current_tag
    } else {
        false
    };

    // Only check for release notes if there is a new update.
    let release_notes = if should_show_update {
        new_release_tag
            .as_ref()
            .and_then(|new_release_tag| get_release_notes(current_tag, new_release_tag))
    } else {
        None
    };

    // TODO: Log instead.
    println!("Check for new release: {:?}", start.elapsed());

    LatestReleaseInfo {
        update_check_time,
        new_release_tag,
        release_notes,
        should_show_update,
    }
}

// TODO: Test this.
fn should_check_for_release(
    previous_update_check_time: Option<DateTime<Utc>>,
    current_time: DateTime<Utc>,
) -> bool {
    if let Some(previous_update_check_time) = previous_update_check_time {
        // Check at most once per day.
        current_time.date_naive() > previous_update_check_time.date_naive()
    } else {
        true
    }
}

fn get_latest_release() -> Option<Release> {
    let rt = tokio::runtime::Runtime::new().ok()?;
    let _guard = rt.enter();
    rt.block_on(
        octocrab::instance()
            .repos("ScanMountGoat", "ssbh_editor")
            .releases()
            .get_latest(),
    )
    .ok()
}

fn get_release_notes(current_tag: &str, latest_tag: &str) -> Option<String> {
    let changelog = reqwest::blocking::get(
        "https://raw.githubusercontent.com/ScanMountGoat/ssbh_editor/main/CHANGELOG.md",
    )
    .ok()?
    .text()
    .ok()?;

    // Find the sections after the current version.
    let start = changelog.find(&format!("## {latest_tag}"))?;
    let end = changelog.find(&format!("## {current_tag}"))?;
    changelog.get(start..end).map(|s| s.to_string())
}
