use crate::{
    EditorResponse, FileResult,
    app::{AnimEditorState, HlpbEditorState, MeshEditorState, ModlEditorState, SkelEditorState},
    editors::{
        adj::adj_editor, anim::anim_editor, hlpb::hlpb_editor, mesh::mesh_editor,
        meshex::meshex_editor, modl::modl_editor, skel::skel_editor,
    },
    model_folder::{FileChanged, ModelFolderState},
};
use egui::Context;
use ssbh_data::prelude::*;
use ssbh_wgpu::ModelFiles;

pub mod adj;
pub mod anim;
pub mod hlpb;
pub mod matl;
pub mod mesh;
pub mod meshex;
pub mod modl;
pub mod nutexb;
pub mod skel;

/// The logic required to open and close an editor window from an open file index.
pub trait Editor {
    type EditorState;

    // TODO: Find a way to simplify these parameters.
    // Merge the open index with the editor state?
    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        dark_mode: bool,
    ) -> Option<EditorResponse>;

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize);
}

impl Editor for AdjData {
    type EditorState = ();

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        _: &mut Self::EditorState,
        _: bool,
    ) -> Option<EditorResponse> {
        let (name, adj) = get_file_to_edit(&mut model.model.adjs, *open_file_index)?;
        let mut adj_to_edit = adj.clone();
        let response = adj_editor(
            ctx,
            &model.folder_path,
            name,
            &mut adj_to_edit,
            find_file(&model.model.meshes, "model.numshb"),
            model
                .validation
                .adj_errors
                .get(open_file_index.as_ref()?)
                .map(|e| e.as_slice())
                .unwrap_or_default(),
        );
        Some(apply_edits(adj, adj_to_edit, response))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.adjs[index]);
    }
}

impl Editor for HlpbData {
    type EditorState = HlpbEditorState;

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        _: bool,
    ) -> Option<EditorResponse> {
        let (name, hlpb) = get_file_to_edit(&mut model.model.hlpbs, *open_file_index)?;
        let mut hlpb_to_edit = hlpb.clone();
        let response = hlpb_editor(
            ctx,
            &model.folder_path,
            name,
            &mut hlpb_to_edit,
            find_file(&model.model.skels, "model.nusktb"),
            state,
        );
        Some(apply_edits(hlpb, hlpb_to_edit, response))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.hlpbs[index])
    }
}

impl Editor for SkelData {
    type EditorState = SkelEditorState;

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        dark_mode: bool,
    ) -> Option<EditorResponse> {
        let (name, skel) = get_file_to_edit(&mut model.model.skels, *open_file_index)?;
        let mut skel_to_edit = skel.clone();
        let response = skel_editor(
            ctx,
            &model.folder_path,
            name,
            &mut skel_to_edit,
            state,
            dark_mode,
        );
        Some(apply_edits(skel, skel_to_edit, response))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.skels[index])
    }
}

impl Editor for AnimData {
    type EditorState = AnimEditorState;

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        _: bool,
    ) -> Option<EditorResponse> {
        let (name, anim) = get_file_to_edit(&mut model.model.anims, *open_file_index)?;
        let mut anim_to_edit = anim.clone();
        let response = anim_editor(ctx, &model.folder_path, name, &mut anim_to_edit, state);
        Some(apply_edits(anim, anim_to_edit, response))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.anims[index])
    }
}

impl Editor for MeshExData {
    type EditorState = ();

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        _: &mut Self::EditorState,
        _: bool,
    ) -> Option<EditorResponse> {
        let (name, meshex) = get_file_to_edit(&mut model.model.meshexes, *open_file_index)?;
        let mut meshex_to_edit = meshex.clone();
        let response = meshex_editor(
            ctx,
            &model.folder_path,
            name,
            &mut meshex_to_edit,
            find_file(&model.model.meshes, "model.numshb"),
        );
        Some(apply_edits(meshex, meshex_to_edit, response))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.meshexes[index])
    }
}

impl Editor for MeshData {
    type EditorState = MeshEditorState;

    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        dark_mode: bool,
    ) -> Option<EditorResponse> {
        let (name, mesh) = get_file_to_edit(&mut model.model.meshes, *open_file_index)?;
        let mut mesh_to_edit = mesh.clone();
        let response = mesh_editor(
            ctx,
            &model.folder_path,
            name,
            &mut mesh_to_edit,
            find_file(&model.model.skels, "model.nusktb"),
            model
                .validation
                .mesh_errors
                .get(open_file_index.as_ref()?)
                .map(|e| e.as_slice())
                .unwrap_or_default(),
            dark_mode,
            state,
        );
        Some(apply_edits(mesh, mesh_to_edit, response))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.meshes[index])
    }
}

impl Editor for ModlData {
    type EditorState = ModlEditorState;
    fn editor(
        ctx: &Context,
        model: &mut ModelFolderState,
        open_file_index: &mut Option<usize>,
        state: &mut Self::EditorState,
        dark_mode: bool,
    ) -> Option<EditorResponse> {
        let (name, modl) = get_file_to_edit(&mut model.model.modls, *open_file_index)?;
        let mut modl_to_edit = modl.clone();
        let response = modl_editor(
            ctx,
            &model.folder_path,
            name,
            &mut modl_to_edit,
            find_file(&model.model.meshes, "model.numshb"),
            find_file(&model.model.matls, "model.numatb"),
            model
                .validation
                .modl_errors
                .get(open_file_index.as_ref()?)
                .map(|e| e.as_slice())
                .unwrap_or_default(),
            state,
            dark_mode,
        );
        Some(apply_edits(modl, modl_to_edit, response))
    }

    fn set_changed(response: &EditorResponse, changed: &mut FileChanged, index: usize) {
        response.set_changed(&mut changed.modls[index])
    }
}

pub fn apply_edits<T: PartialEq>(
    original: &mut T,
    edited: T,
    response: EditorResponse,
) -> EditorResponse {
    let changed = &edited != original;
    *original = edited;
    EditorResponse {
        changed,
        ..response
    }
}

fn get_file_to_edit<T>(
    files: &mut ModelFiles<T>,
    index: Option<usize>,
) -> Option<(&mut String, &mut T)> {
    index
        .and_then(|index| files.get_mut(index))
        .and_then(|(name, file)| Some((name, file.as_mut()?)))
}

fn find_file<'a, T>(files: &'a [(String, FileResult<T>)], name: &str) -> Option<&'a T> {
    files
        .iter()
        .find(|(f, _)| f == name)
        .and_then(|(_, m)| m.as_ref())
}
