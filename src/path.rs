use directories::ProjectDirs;
use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};

pub static PROJECT_DIR: Lazy<ProjectDirs> = Lazy::new(|| {
    // TODO: Avoid unwrap.
    ProjectDirs::from("", "", "ssbh_editor").unwrap()
});

pub fn application_dir() -> &'static Path {
    PROJECT_DIR.data_local_dir()
}

pub fn last_update_check_file() -> PathBuf {
    PROJECT_DIR.data_local_dir().join("update_time.txt")
}

pub fn presets_file() -> PathBuf {
    PROJECT_DIR.data_local_dir().join("presets.json")
}

pub fn preferences_file() -> PathBuf {
    PROJECT_DIR.data_local_dir().join("preferences.json")
}
