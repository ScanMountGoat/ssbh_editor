use crate::path::last_update_check_file;
use chrono::{DateTime, Utc};
use octocrab::models::repos::Release;

pub struct LatestReleaseInfo {
    pub update_check_time: DateTime<Utc>,
    pub new_release_tag: Option<String>,
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
    let should_show_update = if let Some(new_release_tag) = &new_release_tag {
        new_release_tag.as_str() > env!("CARGO_PKG_VERSION")
    } else {
        false
    };
    // TODO: Log instead.
    println!("Check for new release: {:?}", start.elapsed());

    LatestReleaseInfo {
        update_check_time,
        new_release_tag,
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

// TODO: Display a changelog from the repository.
fn get_latest_release() -> Option<Release> {
    let octocrab = octocrab::instance();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?
        .block_on(
            octocrab
                .repos("ScanMountGoat", "ssbh_editor")
                .releases()
                .get_latest(),
        )
        .ok()
}
