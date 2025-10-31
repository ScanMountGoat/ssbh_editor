use crate::path::last_update_check_file;
use chrono::{DateTime, Utc};
use log::error;
use octocrab::models::repos::Release;

pub struct LatestReleaseInfo {
    pub update_check_time: DateTime<Utc>,
    pub new_release: Option<NewRelease>,
    // TODO: Move to UI state.
    pub should_show_update: bool,
}

pub struct NewRelease {
    pub tag: String,
    pub release_notes: Option<String>,
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
    let start = std::time::Instant::now();
    let should_check_for_update =
        should_check_for_release(previous_update_check_time, update_check_time);
    let new_release_tag = if should_check_for_update {
        get_latest_release().map(|r| r.tag_name)
    } else {
        None
    };

    let current_tag = env!("CARGO_PKG_VERSION");
    let (should_show_update, new_release) = match new_release_tag {
        Some(new_tag) => {
            // Only generate release notes if there is a new update.
            let should_show_update = is_new_version(current_tag, &new_tag);
            let new_release = if should_show_update {
                let release_notes = get_release_notes(current_tag, &new_tag);
                Some(NewRelease {
                    tag: new_tag,
                    release_notes,
                })
            } else {
                None
            };
            (should_show_update, new_release)
        }
        None => (false, None),
    };

    // TODO: Log instead.
    println!("Check for new release: {:?}", start.elapsed());

    LatestReleaseInfo {
        update_check_time,
        new_release,
        should_show_update,
    }
}

fn is_new_version(current_tag: &str, new_tag: &str) -> bool {
    // Use proper version comparisons instead of string comparisons.
    let current_tag_version = match semver::Version::parse(current_tag) {
        Ok(v) => v,
        Err(e) => {
            error!("Error parsing current version {current_tag:?}: {e}");
            return false;
        }
    };
    let new_tag_version = match semver::Version::parse(new_tag) {
        Ok(v) => v,
        Err(e) => {
            error!("Error parsing new version {new_tag:?}: {e}");
            return false;
        }
    };

    new_tag_version > current_tag_version
}

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
    .inspect_err(|e| {
        error!("Failed to get latest GitHub release: {e}");
    })
    .ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_release_version_comparison() {
        assert!(!is_new_version("0.10.10", "0.10.9"));
        assert!(!is_new_version("0.11.1", "0.9.2"));
        assert!(is_new_version("0.10.9", "0.10.10"));
        assert!(is_new_version("0.9.2", "0.11.1"));
        assert!(is_new_version("0.0.1", "0.1.0"));
    }
}
