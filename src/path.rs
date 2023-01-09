use crate::model_folder::ModelFolderState;
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

pub fn folder_editor_title(folder_name: &Path, file_name: &str) -> String {
    // Show a simplified version of the path.
    // fighter/mario/motion/body/c00/model.numatb -> c00/model.numatb
    format!(
        "{}/{}",
        Path::new(folder_name)
            .file_name()
            .map(|f| f.to_string_lossy())
            .unwrap_or_default(),
        file_name
    )
}

pub fn folder_display_name(model: &ModelFolderState) -> String {
    // Get enough components to differentiate folder paths.
    // fighter/mario/motion/body/c00 -> mario/motion/body/c00
    let path = Path::new(&model.folder_path)
        .components()
        .rev()
        .take(4)
        .fold(PathBuf::new(), |acc, x| Path::new(&x).join(acc));
    let path = path.to_string_lossy();

    // TODO: Change the icon when expanded.
    format!("🗀 {path}")
}
