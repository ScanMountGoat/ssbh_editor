use std::path::{Path, PathBuf};

use ssbh_wgpu::{swing::SwingPrc, ModelFolder, SharedRenderData};

use crate::{validation::ModelValidationErrors, Thumbnail};

pub struct ModelFolderState {
    pub folder_path: PathBuf,
    pub model: ModelFolder,
    pub thumbnails: Vec<Thumbnail>,
    pub validation: ModelValidationErrors,
    pub changed: FileChanged,
    pub swing_prc: Option<SwingPrc>, // TODO: Add animation slots?
    pub is_meshlist_open: bool,
}

impl ModelFolderState {
    pub fn from_model_and_swing(
        folder_path: PathBuf,
        model: ModelFolder,
        swing_prc: Option<SwingPrc>,
    ) -> Self {
        let changed = FileChanged::from_model(&model);
        Self {
            folder_path,
            model,
            thumbnails: Vec::new(),
            validation: ModelValidationErrors::default(),
            changed,
            swing_prc,
            is_meshlist_open: true,
        }
    }

    pub fn validate(&mut self, shared_data: &SharedRenderData) {
        self.validation = ModelValidationErrors::from_model(
            &self.model,
            shared_data.database(),
            shared_data
                .default_textures()
                .iter()
                .map(|(f, _, d)| (f, d.into())),
        );
    }

    pub fn is_model_folder(&self) -> bool {
        // Check for files used for mesh rendering.
        !self.model.meshes.is_empty()
            || !self.model.modls.is_empty()
            || !self.model.skels.is_empty()
            || !self.model.matls.is_empty()
    }

    pub fn reload(&mut self) {
        // Make sure the ModelFolder is updated first.
        self.model = ModelFolder::load_folder(&self.folder_path);
        self.changed = FileChanged::from_model(&self.model);
    }
}

#[derive(Debug, Default)]
pub struct FileChanged {
    pub meshes: Vec<bool>,
    pub meshexes: Vec<bool>,
    pub skels: Vec<bool>,
    pub matls: Vec<bool>,
    pub modls: Vec<bool>,
    pub adjs: Vec<bool>,
    pub anims: Vec<bool>,
    pub hlpbs: Vec<bool>,
    pub nutexbs: Vec<bool>,
}

impl FileChanged {
    pub fn from_model(model: &ssbh_wgpu::ModelFolder) -> Self {
        Self {
            meshes: vec![false; model.meshes.len()],
            meshexes: vec![false; model.meshexes.len()],
            skels: vec![false; model.skels.len()],
            matls: vec![false; model.matls.len()],
            modls: vec![false; model.modls.len()],
            adjs: vec![false; model.adjs.len()],
            anims: vec![false; model.anims.len()],
            hlpbs: vec![false; model.hlpbs.len()],
            nutexbs: vec![false; model.nutexbs.len()],
        }
    }
}

pub fn find_anim_folders<'a>(
    model: &ModelFolderState,
    anim_folders: &'a [ModelFolderState],
) -> Vec<(usize, &'a ModelFolderState)> {
    find_folders_by_path_affinity(model, anim_folders, |m| !m.model.anims.is_empty())
}

pub fn find_swing_folders<'a>(
    model: &ModelFolderState,
    anim_folders: &'a [ModelFolderState],
) -> Vec<(usize, &'a ModelFolderState)> {
    find_folders_by_path_affinity(model, anim_folders, |m| m.swing_prc.is_some())
}

fn find_folders_by_path_affinity<'a, P: Fn(&'a ModelFolderState) -> bool>(
    model: &ModelFolderState,
    folders: &'a [ModelFolderState],
    predicate: P,
) -> Vec<(usize, &'a ModelFolderState)> {
    let mut folders: Vec<_> = folders
        .iter()
        .enumerate()
        .filter(|(_, m)| predicate(m))
        .collect();

    // Sort in increasing order of affinity with the model folder.
    folders.sort_by_key(|(_, a)| {
        // The folder affinity is the number of matching path components.
        // Consider the model folder "/mario/model/body/c00".
        // The folder "/mario/motion/body/c00" scores higher than "/mario/motion/pump/c00".
        Path::new(&model.folder_path)
            .components()
            .rev()
            .zip(Path::new(&a.folder_path).components().rev())
            .take_while(|(a, b)| a == b)
            .count()
    });
    folders
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model_folder::FileChanged, validation::ModelValidationErrors};
    use ssbh_data::anim_data::AnimData;
    use ssbh_wgpu::ModelFolder;

    fn model_folder(folder_path: PathBuf) -> ModelFolderState {
        ModelFolderState {
            folder_path,
            model: ModelFolder {
                meshes: Vec::new(),
                skels: Vec::new(),
                matls: Vec::new(),
                modls: Vec::new(),
                adjs: Vec::new(),
                anims: Vec::new(),
                hlpbs: Vec::new(),
                nutexbs: Vec::new(),
                meshexes: Vec::new(),
                xmbs: Vec::new(),
            },
            swing_prc: None,
            thumbnails: Vec::new(),
            validation: ModelValidationErrors::default(),
            changed: FileChanged::default(),
            is_meshlist_open: true,
        }
    }

    fn anim_folder(folder_path: PathBuf) -> ModelFolderState {
        ModelFolderState {
            folder_path,
            model: ModelFolder {
                meshes: Vec::new(),
                skels: Vec::new(),
                matls: Vec::new(),
                modls: Vec::new(),
                adjs: Vec::new(),
                anims: vec![(
                    String::new(),
                    Ok(AnimData {
                        major_version: 2,
                        minor_version: 0,
                        final_frame_index: 0.0,
                        groups: Vec::new(),
                    }),
                )],
                hlpbs: Vec::new(),
                nutexbs: Vec::new(),
                meshexes: Vec::new(),
                xmbs: Vec::new(),
            },
            swing_prc: None,
            thumbnails: Vec::new(),
            validation: ModelValidationErrors::default(),
            changed: FileChanged::default(),
            is_meshlist_open: true,
        }
    }

    #[test]
    fn find_anim_folders_no_folders() {
        assert!(find_anim_folders(&model_folder("/model/body/c00".into()), &[]).is_empty());
    }

    #[test]
    fn find_anim_folders_empty_folders() {
        // Folders without animations should be excluded.
        assert!(find_anim_folders(
            &model_folder("/model/body/c00".into()),
            &[model_folder("/motion/body/c00".into())]
        )
        .is_empty());
    }

    #[test]
    fn find_anim_folders_compare_matches() {
        // The second folder is the best match.
        let anim_folders = vec![
            anim_folder("/motion/pump/c00".into()),
            anim_folder("/motion/body/c00".into()),
            anim_folder("/motion/body/c01".into()),
        ];
        let folders = find_anim_folders(&model_folder("/model/body/c00".into()), &anim_folders);
        assert!(matches!(folders.as_slice(), [(2, _), (0, _), (1, _)]));
    }
}
