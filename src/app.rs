use crate::{
    load_model, load_models_recursive,
    material::{
        add_parameters, apply_preset, default_material, missing_parameters, remove_parameters,
        unused_parameters,
    },
    widgets::*,
};
use egui::{CollapsingHeader, ScrollArea};
use lazy_static::lazy_static;
use log::{error, Log};
use rfd::FileDialog;
use ssbh_data::{matl_data::MatlEntryData, modl_data::ModlEntryData, prelude::*};
use ssbh_wgpu::{ModelFolder, PipelineData, RenderModel, RenderSettings, ShaderDatabase};
use std::{path::Path, sync::Mutex};

lazy_static! {
    pub static ref LOGGER: AppLogger = AppLogger {
        messages: Mutex::new(Vec::new())
    };
}

pub struct AppLogger {
    messages: Mutex<Vec<(log::Level, String)>>,
}

impl Log for AppLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // Only show logs from the application for now.
        // TODO: Log ssbh_wgpu and wgpu info?
        metadata.level() <= log::Level::Warn && metadata.target() == "ssbh_editor"
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            self.messages
                .lock()
                .unwrap()
                .push((record.level(), format!("{}", record.args())));
        }
    }

    fn flush(&self) {}
}

pub struct SsbhApp {
    pub should_refresh_meshes: bool,
    pub should_refresh_render_settings: bool,

    pub should_show_update: bool,
    pub new_release_tag: Option<String>,

    pub material_presets: Vec<MatlEntryData>,

    pub ui_state: UiState,
    // TODO: How to manage the thumbnail cache?
    // TODO: Is parallel list with models the best choice here?
    pub models: Vec<ModelFolder>,
    pub render_models: Vec<RenderModel>,
    pub thumbnails: Vec<Vec<(String, egui::TextureId)>>,
    pub default_thumbnails: Vec<(String, egui::TextureId)>,
    pub animation_state: AnimationState,
    pub render_state: RenderState,
}

pub struct UiState {
    // TODO: Allow more than one open editor of each type?
    pub material_editor_open: bool,
    pub render_settings_open: bool,
    pub right_panel_tab: PanelTab,
    pub matl_editor_advanced_mode: bool,
    pub modl_editor_advanced_mode: bool,
    pub preset_window_open: bool,
    pub selected_material_preset_index: usize,

    // TODO: Is there a better way to track this?
    // Clicking an item in the file list sets the selected index.
    // If the index is not None, the corresponding editor stays open.
    pub selected_folder_index: Option<usize>,
    pub selected_skel_index: Option<usize>,
    pub selected_hlpb_index: Option<usize>,
    pub selected_matl_index: Option<usize>,
    pub selected_modl_index: Option<usize>,
    pub selected_mesh_index: Option<usize>,
    pub selected_material_index: usize,
}

pub struct AnimationState {
    pub current_frame: f32,
    pub is_playing: bool,
    pub animation_frame_was_changed: bool,
    pub selected_slot: usize,
    pub animations: Vec<(String, AnimData)>,
    pub previous_frame_start: std::time::Instant,
}

pub struct RenderState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub default_textures: Vec<(String, wgpu::Texture)>,
    pub stage_cube: (wgpu::TextureView, wgpu::Sampler),
    pub pipeline_data: PipelineData,
    pub render_settings: RenderSettings,
    pub shader_database: ShaderDatabase,
}

// Keep track of what UI should be displayed.
#[derive(PartialEq, Eq)]
pub enum PanelTab {
    MeshList,
    AnimList,
}

impl SsbhApp {
    fn clear_workspace(&mut self) {
        self.models = Vec::new();
        self.render_models = Vec::new();
        self.thumbnails = Vec::new();
        self.animation_state.animations = Vec::new();
        // TODO: Reset selected indices?
        // TODO: Is there an easy way to write this?
    }
}

const ICON_SIZE: f32 = 16.0;

impl epi::App for SsbhApp {
    fn name(&self) -> &str {
        "SSBH Editor"
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Open Folder...").clicked() {
                        ui.close_menu();

                        if let Some(folder) = FileDialog::new().pick_folder() {
                            // TODO: Sort alphabetically?
                            // TODO: Allow for opening folders with no mesh/modl?
                            self.models = load_models_recursive(folder);
                            self.should_refresh_meshes = true;
                        }
                    }

                    if ui.button("Add Folder to Workspace...").clicked() {
                        ui.close_menu();

                        if let Some(folder) = FileDialog::new().pick_folder() {
                            // Load the folder manually to avoid skipping folders with just animations.
                            // TODO: Is there an easier way to allow loading animation folders?
                            let new_model = load_model(&folder);
                            self.models.push(new_model);
                            self.should_refresh_meshes = true;
                        }
                    }

                    if ui.button("Clear Workspace").clicked() {
                        ui.close_menu();

                        self.clear_workspace();
                    }
                });

                egui::menu::menu_button(ui, "Menu", |ui| {
                    if ui.button("Render Settings").clicked() {
                        ui.close_menu();

                        self.ui_state.render_settings_open = true;
                    }
                });
            });
        });

        // Add windows here so they can overlap everything except the top panel.
        // We store some state in self to keep track of whether this should be left open.
        if self.ui_state.render_settings_open {
            self.should_refresh_render_settings = true;
        }
        render_settings(
            ctx,
            &mut self.render_state.render_settings,
            &mut self.ui_state.render_settings_open,
        );

        // Don't reopen the window once closed.
        if self.should_show_update {
            if let Some(new_release_tag) = &self.new_release_tag {
                egui::Window::new("New Release Available")
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .resizable(false)
                    .collapsible(false)
                    .open(&mut self.should_show_update)
                    .show(ctx, |ui| {
                        ui.label("A new release of SSBH Editor is available!");
                        ui.label(format!(
                            "The latest version is {}. The current version is {}.",
                            new_release_tag,
                            env!("CARGO_PKG_VERSION")
                        ));
                        ui.label("Download the new version from here:");
                        // TODO: How to open the hyperlink?
                        let release_link = "https://github.com/ScanMountGoat/ssbh_editor/releases";
                        if ui.hyperlink(release_link).clicked() {
                            // TODO: Log errors?
                            if let Err(open_err) = open::that(release_link) {
                                log::error!(
                                    "Failed to open link ({release_link}) to releases {open_err}"
                                );
                            }
                        }
                        // TODO: Show latest version and release notes.
                        // TODO: Parse release notes from changelog.
                    });
            }
        }

        let display_name = |folder: &str, name: &str| {
            format!(
                "{}/{}",
                Path::new(folder).file_name().unwrap().to_string_lossy(),
                name
            )
        };

        // TODO: Refactor this so they can also be "docked" in side panel tabs.
        // The functions should take an additional ui parameter.
        if let Some(folder_index) = self.ui_state.selected_folder_index {
            if let Some(model) = self.models.get_mut(folder_index) {
                if let Some(skel_index) = self.ui_state.selected_skel_index {
                    if let Some((name, skel)) = model.skels.get_mut(skel_index) {
                        if !skel_editor(ctx, &display_name(&model.folder_name, name), skel) {
                            // Close the window.
                            self.ui_state.selected_skel_index = None;
                        }
                    }
                }

                if let Some(mesh_index) = self.ui_state.selected_mesh_index {
                    if let Some((name, mesh)) = model.meshes.get_mut(mesh_index) {
                        if !mesh_editor(ctx, &display_name(&model.folder_name, name), mesh) {
                            // Close the window.
                            self.ui_state.selected_mesh_index = None;
                        }
                    }
                }

                // TODO: Make all this code a function?
                if let Some(matl_index) = self.ui_state.selected_matl_index {
                    if let Some((name, matl)) = model.matls.get_mut(matl_index) {
                        // TODO: Fix potential crash if thumbnails aren't present.
                        // TODO: Make this a method to simplify arguments.
                        if !matl_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            &mut self.ui_state,
                            matl,
                            model
                                .modls
                                .iter_mut()
                                .find(|(f, _)| f == "model.numdlb")
                                .map(|(_, m)| m),
                            &self.thumbnails[folder_index],
                            &self.default_thumbnails,
                            &self.render_state.shader_database,
                            &self.material_presets,
                        ) {
                            // Close the window.
                            self.ui_state.selected_matl_index = None;
                        } else if let Some(render_model) = self.render_models.get_mut(folder_index)
                        {
                            // TODO: Add change tracking using .changed() to improve performance.
                            // TODO: Is it worth optimizing this to only effect certain materials?
                            // Only the model.numatb is rendered in the viewport for now.
                            if name == "model.numatb" {
                                // TODO: How to efficiently handle renaming materials in ssbh_wgpu?
                                render_model.update_materials(
                                    &self.render_state.device,
                                    &self.render_state.queue,
                                    &matl.entries,
                                    &self.render_state.pipeline_data,
                                    &self.render_state.default_textures,
                                    &self.render_state.stage_cube,
                                    &self.render_state.shader_database,
                                );
                            }
                        }
                    }
                }

                if let Some(modl_index) = self.ui_state.selected_modl_index {
                    if let Some((name, modl)) = model.modls.get_mut(modl_index) {
                        let matl = model
                            .matls
                            .iter()
                            .find(|(f, _)| f == "model.numatb")
                            .map(|(_, m)| m);
                        if !modl_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            modl,
                            model
                                .meshes
                                .iter()
                                .find(|(f, _)| f == "model.numshb")
                                .map(|(_, m)| m),
                            matl,
                            &mut self.ui_state.modl_editor_advanced_mode,
                        ) {
                            // Close the window.
                            self.ui_state.selected_modl_index = None;
                        } else if let Some(render_model) = self.render_models.get_mut(folder_index)
                        {
                            // Update material previews in the viewport if the window remains open.
                            // TODO: Is it worth only doing this when changes actually occur?
                            render_model.reassign_materials(modl, matl);
                        }
                    }
                }

                if let Some(hlpb_index) = self.ui_state.selected_hlpb_index {
                    if let Some((name, hlpb)) = model.hlpbs.get_mut(hlpb_index) {
                        if !hlpb_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            hlpb,
                            model
                                .skels
                                .iter()
                                .find(|(f, _)| f == "model.nusktb")
                                .map(|(_, s)| s),
                        ) {
                            // Close the window.
                            self.ui_state.selected_hlpb_index = None;
                        }
                    }
                }
            }
        }

        egui::SidePanel::left("left_panel")
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Files");

                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for (folder_index, model) in self.models.iter_mut().enumerate() {
                            // TODO: Will these IDs be unique?
                            // TODO: Create unique IDs without displaying indices on folder names.
                            CollapsingHeader::new(format!(
                                "{}.{}",
                                Path::new(&model.folder_name)
                                    .file_name()
                                    .unwrap()
                                    .to_string_lossy(),
                                folder_index
                            ))
                            .default_open(true)
                            .show(ui, |ui| {
                                // TODO: Show a visual indication if key files like skel or modl are missing?
                                // TODO: Show a visual indication if a file has warnings/errors.
                                // This will encourage users to click and investigate the errors.

                                // Clicking a file opens the corresponding editor.
                                // Set selected index so the editor remains open for the file.
                                // TODO: Should the index be cleared when reloading models?
                                // TODO: How to reuse code?
                                list_files(
                                    ui,
                                    &model.meshes,
                                    folder_index,
                                    &mut self.ui_state.selected_folder_index,
                                    &mut self.ui_state.selected_mesh_index,
                                    Some("model.numshb"),
                                );

                                list_files(
                                    ui,
                                    &model.skels,
                                    folder_index,
                                    &mut self.ui_state.selected_folder_index,
                                    &mut self.ui_state.selected_skel_index,
                                    Some("model.nusktb"),
                                );

                                list_files(
                                    ui,
                                    &model.hlpbs,
                                    folder_index,
                                    &mut self.ui_state.selected_folder_index,
                                    &mut self.ui_state.selected_hlpb_index,
                                    None,
                                );

                                list_files(
                                    ui,
                                    &model.matls,
                                    folder_index,
                                    &mut self.ui_state.selected_folder_index,
                                    &mut self.ui_state.selected_matl_index,
                                    Some("model.numatb"),
                                );

                                list_files(
                                    ui,
                                    &model.modls,
                                    folder_index,
                                    &mut self.ui_state.selected_folder_index,
                                    &mut self.ui_state.selected_modl_index,
                                    Some("model.numdlb"),
                                );

                                for (_i, (name, anim)) in model.anims.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        empty_icon(ui);
                                        if ui.button(name).clicked() {
                                            self.ui_state.selected_folder_index =
                                                Some(folder_index);
                                            // TODO: Store (folder_index, anim_index) to avoid cloning?
                                            let new_animation = (name.to_string(), anim.clone());
                                            if self.animation_state.animations.is_empty() {
                                                self.animation_state.animations.push(new_animation);
                                            } else if let Some(slot) = self
                                                .animation_state
                                                .animations
                                                .get_mut(self.animation_state.selected_slot)
                                            {
                                                *slot = new_animation;
                                            }

                                            // Preview the new animation as soon as it is clicked.
                                            self.animation_state.animation_frame_was_changed = true;
                                        }
                                    });
                                }

                                // TODO: Display larger versions when clicking?
                                // TODO: How to manage the thumbnails?
                                // TODO: Cube map thumbnails.
                                for (file, nutexb) in model.nutexbs.iter() {
                                    ui.horizontal(|ui| {
                                        if let Some(model_thumbnails) =
                                            self.thumbnails.get(folder_index)
                                        {
                                            if let Some((_, thumbnail)) = model_thumbnails
                                                .iter()
                                                .find(|(name, _)| name == file)
                                            {
                                                ui.image(
                                                    *thumbnail,
                                                    egui::Vec2::new(ICON_SIZE, ICON_SIZE),
                                                );
                                            }
                                        }
                                        // TODO: Create a proper nutexb viewer.
                                        ui.label(file)
                                            .on_hover_text(format!("{:#?}", nutexb.footer));
                                    });
                                }
                            });
                        }
                    });
            });

        egui::SidePanel::right("right panel")
            .min_width(350.0)
            .show(ctx, |ui| {
                // TODO: Is it worth creating a dedicated tab control?
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.ui_state.right_panel_tab,
                        PanelTab::MeshList,
                        egui::RichText::new("Meshes").heading(),
                    );
                    ui.selectable_value(
                        &mut self.ui_state.right_panel_tab,
                        PanelTab::AnimList,
                        egui::RichText::new("Animations").heading(),
                    );
                });

                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| match self.ui_state.right_panel_tab {
                        PanelTab::MeshList => mesh_list(self, ui),
                        PanelTab::AnimList => anim_list(self, ui),
                    });
            });

        egui::TopBottomPanel::bottom("bottom panel").show(ctx, |ui| {
            self.animation_bar(ui);
            // TODO: Add a proper log window similar to Blender's info log.
            // Each entry should contain details on where the error occurred.
            // Clicking should expand to show the full log.
            // Show the most recently logged item + log level icon.
            if let Some((level, message)) = LOGGER.messages.lock().unwrap().first() {
                ui.horizontal(|ui| {
                    match level {
                        log::Level::Error => error_icon(ui),
                        log::Level::Warn => warning_icon(ui),
                        log::Level::Info => (),
                        log::Level::Debug => (),
                        log::Level::Trace => (),
                    }
                    ui.label(message);
                });
            }
        });
    }
}

impl SsbhApp {
    fn animation_bar(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(
            egui::Layout::left_to_right().with_cross_align(egui::Align::Center),
            |ui| {
                let mut final_frame_index = 0.0;
                for (_, a) in &self.animation_state.animations {
                    if a.final_frame_index > final_frame_index {
                        final_frame_index = a.final_frame_index;
                    }
                }

                // TODO: How to fill available space?
                ui.spacing_mut().slider_width = (ui.available_size().x - 120.0).max(0.0);
                if ui
                    .add(
                        egui::Slider::new(
                            &mut self.animation_state.current_frame,
                            0.0..=final_frame_index,
                        )
                        .step_by(1.0),
                    )
                    .changed()
                {
                    // Manually trigger an update in case the playback is paused.
                    self.animation_state.animation_frame_was_changed = true;
                }
                // Nest these conditions to avoid displaying both "Pause" and "Play" at once.
                let size = [60.0, 30.0];
                if self.animation_state.is_playing {
                    if ui.add_sized(size, egui::Button::new("Pause")).clicked() {
                        self.animation_state.is_playing = false;
                    }
                } else if ui.add_sized(size, egui::Button::new("Play")).clicked() {
                    self.animation_state.is_playing = true;
                }
            },
        );
    }
}

fn list_files<T>(
    ui: &mut egui::Ui,
    files: &[(String, T)],
    folder_index: usize,
    selected_folder_index: &mut Option<usize>,
    selected_file_index: &mut Option<usize>,
    required_file: Option<&'static str>,
) {
    for (i, (name, _)) in files.iter().enumerate() {
        ui.horizontal(|ui| {
            // TODO: How to check for and store validation?
            empty_icon(ui);
            if ui.button(name.clone()).clicked() {
                *selected_folder_index = Some(folder_index);
                *selected_file_index = Some(i);
            }
        });
    }
    if let Some(required_file) = required_file {
        if !files.iter().any(|(f, _)| f == required_file) {
            missing_file(ui, required_file);
        }
    }
}

fn missing_file(ui: &mut egui::Ui, name: &str) {
    // TODO: Should the tooltip cover the entire layout?
    ui.horizontal(|ui| {
        missing_icon(ui);
        ui.add_enabled(
            false,
            egui::Button::new(egui::RichText::new(name).strikethrough()),
        );
    })
    .response
    .on_hover_text(format!("Missing required file {name}"));
}

fn empty_icon(ui: &mut egui::Ui) {
    ui.allocate_space(egui::Vec2::new(ICON_SIZE, ICON_SIZE));
}

fn missing_icon(ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("⚠").size(ICON_SIZE));
}

fn warning_icon(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("⚠")
            .strong()
            .color(egui::Color32::from_rgb(255, 210, 0))
            .size(ICON_SIZE),
    );
}

fn error_icon(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("⚠")
            .strong()
            .color(egui::Color32::from_rgb(200, 40, 40))
            .size(ICON_SIZE),
    );
}

fn mesh_list(app: &mut SsbhApp, ui: &mut egui::Ui) {
    for (i, folder) in app.models.iter().enumerate() {
        // TODO: Will these IDs be unique?
        CollapsingHeader::new(format!(
            "{}.{}",
            std::path::Path::new(&folder.folder_name)
                .file_name()
                .unwrap()
                .to_string_lossy(),
            i
        ))
        .default_open(true)
        .show(ui, |ui| {
            // TODO: How to ensure the render models stay in sync with the model folder?
            if let Some(render_model) = app.render_models.get_mut(i) {
                for mesh in &mut render_model.meshes {
                    ui.add(EyeCheckBox::new(&mut mesh.is_visible, &mesh.name));
                }
            }
        });
    }
}

fn anim_list(app: &mut SsbhApp, ui: &mut egui::Ui) {
    // TODO: Will these IDs be unique?
    if ui.button("Add Slot").clicked() {
        app.animation_state.animations.push((
            "EMPTY".to_string(),
            AnimData {
                major_version: 2,
                minor_version: 0,
                final_frame_index: 0.0,
                groups: Vec::new(),
            },
        ));
    }

    let mut slots_to_remove = Vec::new();
    for (i, (name, _anim)) in app.animation_state.animations.iter().enumerate().rev() {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut app.animation_state.selected_slot,
                i,
                format!("Slot {i} - {name}"),
            );
            if ui.button("Remove").clicked() {
                slots_to_remove.push(i);
            }
        });
    }

    // TODO: Force only one slot to be removed?
    for slot in slots_to_remove {
        app.animation_state.animations.remove(slot);
    }
    // for (i, (name, anim)) in app.animation_state.animations.iter().enumerate() {
    //     CollapsingHeader::new(format!("Slot {i} - {name}"))
    //         .default_open(false)
    //         .show(ui, |ui| {
    //             for group in &anim.groups {
    //                 CollapsingHeader::new(group.group_type.to_string())
    //                     .default_open(true)
    //                     .show(ui, |ui| {
    //                         for node in &group.nodes {
    //                             match node.tracks.as_slice() {
    //                                 [_] => {
    //                                     // Don't use a collapsing header if there is only one track.
    //                                     // This simplifies the layout for most boolean and transform tracks.
    //                                     // TODO: How to toggle visibility for rendering?
    //                                     ui.add(EyeCheckBox::new(&mut true, &node.name));
    //                                 }
    //                                 _ => {
    //                                     CollapsingHeader::new(&node.name).default_open(true).show(
    //                                         ui,
    //                                         |ui| {
    //                                             for track in &node.tracks {
    //                                                 // TODO: How to toggle visibility for rendering?
    //                                                 ui.add(EyeCheckBox::new(
    //                                                     &mut true,
    //                                                     &track.name,
    //                                                 ));
    //                                             }
    //                                         },
    //                                     );
    //                                 }
    //                             }
    //                         }
    //                     });
    //             }
    //         });
    // }
}

fn hlpb_editor(
    ctx: &egui::Context,
    title: &str,
    hlpb: &mut HlpbData,
    skel: Option<&SkelData>,
) -> bool {
    let mut open = true;

    egui::Window::new(format!("Hlpb Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Hlpb", &["nuhlpb"])
                            .save_file()
                        {
                            if let Err(e) = hlpb.write_to_file(file) {
                                error!(target: "ssbh_editor", "Failed to save Hlpb (.nuhlpb): {}", e);
                            }
                        }
                    }
                });
            });

            // TODO: Add some sort of separator to make the menu easier to see.
            ui.add(egui::Separator::default().horizontal());

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if !hlpb.aim_constraints.is_empty() {
                        CollapsingHeader::new("Aim Constraints")
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::Grid::new("aim").striped(true).show(ui, |ui| {
                                    ui.label(egui::RichText::new("name").heading());
                                    ui.label(egui::RichText::new("aim 1").heading());
                                    ui.label(egui::RichText::new("aim 2").heading());
                                    ui.label(egui::RichText::new("type 1").heading());
                                    ui.label(egui::RichText::new("type 2").heading());
                                    ui.label(egui::RichText::new("target 1").heading());
                                    ui.label(egui::RichText::new("target 2").heading());
                                    ui.end_row();

                                    for (i, aim) in hlpb.aim_constraints.iter_mut().enumerate() {
                                        ui.label(&aim.name);
                                        bone_combo_box(
                                            ui,
                                            &mut aim.aim_bone_name1,
                                            format!("a{:?}0", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.aim_bone_name2,
                                            format!("a{:?}1", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.aim_type1,
                                            format!("a{:?}2", i),
                                            skel,
                                            &["DEFAULT"],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.aim_type2,
                                            format!("a{:?}3", i),
                                            skel,
                                            &["DEFAULT"],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.target_bone_name1,
                                            format!("a{:?}4", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut aim.target_bone_name2,
                                            format!("a{:?}5", i),
                                            skel,
                                            &[],
                                        );
                                        ui.end_row();
                                    }
                                });
                            });
                    }

                    if !hlpb.orient_constraints.is_empty() {
                        // ui.heading("Orient Constraints");
                        CollapsingHeader::new("Orient Constraints")
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::Grid::new("orient").striped(true).show(ui, |ui| {
                                    ui.label(egui::RichText::new("name").heading());
                                    ui.label(egui::RichText::new("bone").heading());
                                    ui.label(egui::RichText::new("root").heading());
                                    ui.label(egui::RichText::new("parent").heading());
                                    ui.label(egui::RichText::new("driver").heading());
                                    ui.end_row();

                                    for (i, orient) in
                                        hlpb.orient_constraints.iter_mut().enumerate()
                                    {
                                        ui.label(&orient.name);
                                        bone_combo_box(
                                            ui,
                                            &mut orient.bone_name,
                                            format!("o{:?}0", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut orient.root_bone_name,
                                            format!("o{:?}1", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut orient.parent_bone_name,
                                            format!("o{:?}2", i),
                                            skel,
                                            &[],
                                        );
                                        bone_combo_box(
                                            ui,
                                            &mut orient.driver_bone_name,
                                            format!("o{:?}3", i),
                                            skel,
                                            &[],
                                        );
                                        ui.end_row();
                                    }
                                });
                            });
                    }
                });
        });

    open
}

fn modl_editor(
    ctx: &egui::Context,
    title: &str,
    modl: &mut ModlData,
    mesh: Option<&MeshData>,
    matl: Option<&MatlData>,
    advanced_mode: &mut bool,
) -> bool {
    let mut open = true;

    egui::Window::new(format!("Modl Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Modl", &["numdlb"])
                            .save_file()
                        {
                            if let Err(e) = modl.write_to_file(file) {
                                error!(target: "ssbh_editor", "Failed to save Modl (.numdlb): {}", e);
                            }
                        }
                    }
                });
            });

            // TODO: Add some sort of separator to make the menu easier to see.
            ui.add(egui::Separator::default().horizontal());

            // Advanced mode has more detailed information that most users won't want to edit.
            ui.checkbox(advanced_mode, "Advanced Settings");

            // Manually adding entries is error prone, so check for advanced mode.
            if *advanced_mode && ui.button("Add Entry").clicked() {
                // Pick an arbitrary material to make the mesh visible in the viewport.
                let default_material = matl
                    .and_then(|m| m.entries.get(0).map(|e| e.material_label.clone()))
                    .unwrap_or_else(|| String::from("PLACEHOLDER"));

                modl.entries.push(ModlEntryData {
                    mesh_object_name: String::from("PLACEHOLDER"),
                    mesh_object_sub_index: 0,
                    material_label: default_material,
                });
            }

            if let Some(mesh) = mesh {
                // TODO: Optimize this?
                let missing_entries: Vec<_> = mesh
                    .objects
                    .iter()
                    .filter(|mesh| {
                        !modl.entries
                            .iter()
                            .any(|e| {
                                e.mesh_object_name == mesh.name
                                    && e.mesh_object_sub_index == mesh.sub_index
                            })
                    })
                    .collect();

                // Pick an arbitrary material to make the mesh visible in the viewport.
                let default_material = matl
                    .and_then(|m| m.entries.get(0).map(|e| e.material_label.clone()))
                    .unwrap_or_else(|| String::from("PLACEHOLDER"));

                if !missing_entries.is_empty() && ui.button("Add Missing Entries").clicked() {
                    for mesh in missing_entries {
                        modl.entries.push(ModlEntryData {
                            mesh_object_name: mesh.name.clone(),
                            mesh_object_sub_index: mesh.sub_index,
                            material_label: default_material.clone(),
                        });
                    }
                }
            }

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Grid::new("modl_grid").striped(true).show(ui, |ui| {
                        // Header
                        // TODO: There are three ways to display duplicate object names.
                        // 1. "object.0", "object.1"
                        // 2. "object", "object"
                        // 3. heirarchy with "object0" and "object1" as children of "object"
                        ui.heading("Mesh Object");
                        ui.heading("Material Label");
                        ui.end_row();

                        let mut entries_to_remove = Vec::new();
                        for (i, entry) in modl.entries.iter_mut().enumerate() {
                            // TODO: If a user renames a material, both the modl and matl need to update.
                            // TODO: How to handle the case where the user enters a duplicate name?
                            // TODO: module of useful functions from ModelFolder -> ui?
                            if *advanced_mode {
                                mesh_name_combo_box(
                                    ui,
                                    &mut entry.mesh_object_name,
                                    format!("mesh{:?}", i),
                                    mesh,
                                );
                            } else {
                                ui.label(&entry.mesh_object_name);
                            }

                            // TODO: How to handle sub indices?
                            // TODO: Show an indication if the matl is missing the current material.
                            material_label_combo_box(
                                ui,
                                &mut entry.material_label,
                                format!("matl{:?}", i),
                                matl,
                            );

                            if *advanced_mode && ui.button("Delete").clicked() {
                                entries_to_remove.push(i);
                            }
                            ui.end_row();
                        }

                        // TODO: Will this handle multiple entries?
                        for i in entries_to_remove {
                            modl.entries.remove(i);
                        }
                    });
                });
        });

    open
}

fn mesh_name_combo_box(
    ui: &mut egui::Ui,
    mesh_name: &mut String,
    id: impl std::hash::Hash,
    mesh: Option<&MeshData>,
) {
    egui::ComboBox::from_id_source(id)
        .selected_text(mesh_name.clone())
        .width(300.0)
        .show_ui(ui, |ui| {
            // TODO: Just use text boxes if the mesh is missing?
            if let Some(mesh) = mesh {
                for mesh in &mesh.objects {
                    ui.selectable_value(mesh_name, mesh.name.to_string(), &mesh.name);
                }
            }
        });
}

fn material_label_combo_box(
    ui: &mut egui::Ui,
    material_label: &mut String,
    id: impl std::hash::Hash,
    matl: Option<&MatlData>,
) {
    egui::ComboBox::from_id_source(id)
        .selected_text(material_label.clone())
        .width(400.0)
        .show_ui(ui, |ui| {
            // TODO: Just use text boxes if the matl is missing?
            if let Some(matl) = matl {
                for label in matl.entries.iter().map(|e| &e.material_label) {
                    ui.selectable_value(material_label, label.to_string(), label);
                }
            }
        });
}

fn bone_combo_box(
    ui: &mut egui::Ui,
    bone_name: &mut String,
    id: impl std::hash::Hash,
    skel: Option<&SkelData>,
    extra_names: &[&str],
) {
    egui::ComboBox::from_id_source(id)
        .selected_text(bone_name.clone())
        // .width(400.0)
        .show_ui(ui, |ui| {
            // TODO: Just use text boxes if the skel is missing?
            for name in extra_names {
                ui.selectable_value(bone_name, name.to_string(), name.to_string());
            }

            if let Some(skel) = skel {
                for bone in &skel.bones {
                    ui.selectable_value(bone_name, bone.name.clone(), &bone.name);
                }
            }
        });
}

fn mesh_editor(ctx: &egui::Context, title: &str, mesh: &mut MeshData) -> bool {
    let mut open = true;

    egui::Window::new(format!("Mesh Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // TODO: Add options to show a grid or tree based layout?
                    // TODO: Add a toggle for basic or advanced options.
                    ui.heading("Mesh Objects");

                    egui::Grid::new("some_unique_id").show(ui, |ui| {
                        for (_i, mesh_object) in mesh.objects.iter_mut().enumerate() {
                            // TODO: Link name edits with the numdlb and numshexb.
                            // This will need to check for duplicate names.
                            // TODO: Reorder mesh objects?
                            ui.label(&mesh_object.name);
                            ui.label(mesh_object.sort_bias.to_string());
                            ui.checkbox(
                                &mut mesh_object.disable_depth_write,
                                "Disable Depth Write",
                            );
                            ui.checkbox(&mut mesh_object.disable_depth_test, "Disable Depth Test");

                            ui.end_row();
                        }
                    });
                });
        });

    open
}

fn skel_editor(ctx: &egui::Context, title: &str, skel: &mut SkelData) -> bool {
    let mut open = true;

    egui::Window::new(format!("Skel Editor ({title})"))
        .resizable(true)
        .open(&mut open)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::menu::bar(ui, |ui| {
                        egui::menu::menu_button(ui, "File", |ui| {
                            if ui.button("Save").clicked() {
                                ui.close_menu();

                                if let Some(file) = FileDialog::new()
                                    .add_filter("Skel", &["nusktb"])
                                    .save_file()
                                {
                                    if let Err(e) = skel.write_to_file(file) {
                                        error!(target: "ssbh_editor", "Failed to save Skel (.nusktb): {}", e);
                                    }
                                }
                            }
                        });
                    });

                    ui.add(egui::Separator::default().horizontal());

                    // TODO: Add options to show a grid or tree based layout?
                    egui::Grid::new("some_unique_id").show(ui, |ui| {
                        // Header
                        ui.heading("Bone");
                        ui.heading("Parent");
                        ui.end_row();

                        // TODO: Do this without clone?
                        let other_bones = skel.bones.clone();

                        for (i, bone) in skel.bones.iter_mut().enumerate() {
                            ui.label(&bone.name);
                            let parent_bone_name = bone
                                .parent_index
                                .and_then(|i| other_bones.get(i))
                                .map(|p| p.name.as_str())
                                .unwrap_or("None");

                            egui::ComboBox::from_id_source(i)
                                .selected_text(parent_bone_name)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut bone.parent_index, None, "None");
                                    ui.separator();
                                    // TODO: Is there a way to make this not O(N^2)?
                                    for (other_i, other_bone) in other_bones.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut bone.parent_index,
                                            Some(other_i),
                                            &other_bone.name,
                                        );
                                    }
                                });
                            ui.end_row();
                        }
                    });
                });
        });

    open
}

fn render_settings(ctx: &egui::Context, settings: &mut RenderSettings, open: &mut bool) {
    egui::Window::new("Render Settings")
        .open(open)
        .resizable(true)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.heading("Debug Shading");
                    // TODO: Lay these out horizontally?
                    enum_combo_box(ui, "Debug Mode", "Debug Mode", &mut settings.debug_mode);
                    if settings.debug_mode == ssbh_wgpu::DebugMode::Shaded {
                        enum_combo_box(
                            ui,
                            "Transition Material",
                            "Transition Material",
                            &mut settings.transition_material,
                        );
                        ui.label("Transition Factor");
                        ui.add(egui::Slider::new(
                            &mut settings.transition_factor,
                            0.0..=1.0,
                        ));
                    }

                    ui.heading("Render Passes");
                    ui.checkbox(&mut settings.render_diffuse, "Enable Diffuse");
                    ui.checkbox(&mut settings.render_specular, "Enable Specular");
                    ui.checkbox(&mut settings.render_emission, "Enable Emission");
                    ui.checkbox(&mut settings.render_rim_lighting, "Enable Rim Lighting");

                    ui.heading("Lighting");
                    ui.checkbox(&mut settings.render_shadows, "Enable Shadows");
                });
        });
}

#[allow(clippy::too_many_arguments)]
fn matl_editor(
    ctx: &egui::Context,
    title: &str,
    ui_state: &mut UiState,
    matl: &mut MatlData,
    modl: Option<&mut ModlData>,
    folder_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    shader_database: &ShaderDatabase,
    material_presets: &[MatlEntryData],
) -> bool {
    let mut open = true;

    egui::Window::new(format!("Matl Editor ({title})"))
        .open(&mut open)
        .default_size(egui::Vec2::new(400.0, 700.0))
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Matl", &["numatb"])
                            .save_file()
                        {
                            if let Err(e) = matl.write_to_file(file) {
                                error!(target: "ssbh_editor", "Failed to save Matl (.numatb): {}", e);
                            }
                        }
                    }
                });

                egui::menu::menu_button(ui, "Material", |ui| {
                    if ui.button("Add New Material").clicked() {
                        ui.close_menu();

                        // TODO: Select options from presets?
                        let new_entry = default_material();
                        matl.entries.push(new_entry);

                        ui_state.selected_material_index = matl.entries.len() - 1;
                    }

                    if ui.button("Apply Preset").clicked() {
                        ui_state.preset_window_open = true;
                    }
                });
            });

            // TODO: Simplify logic for closing window.
            let mut open = ui_state.preset_window_open;
            egui::Window::new("Select Material Preset")
                .open(&mut ui_state.preset_window_open)
                .show(ctx, |ui| {
                    for (i, preset) in material_presets.iter().enumerate() {
                        ui.selectable_value(&mut ui_state.selected_material_preset_index, i, &preset.material_label);
                    }

                    if ui.button("Apply").clicked() {
                        if let Some(preset) = material_presets.get(ui_state.selected_material_preset_index) {
                            if let Some(entry) = matl.entries.get_mut(ui_state.selected_material_index) {
                                *entry = apply_preset(entry, preset);
                            }
                        }

                        open = false;
                    }
                });
            if !open {
                ui_state.preset_window_open = false;
            }

            ui.add(egui::Separator::default().horizontal());

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Only display a single material at a time.
                    // This avoids cluttering the UI.
                    // TODO: Add the ability to rename materials.
                    ui.horizontal(|ui| {
                        ui.label("Material");
                        egui::ComboBox::from_id_source("MatlEditorMaterialLabel")
                            .width(400.0)
                            .show_index(ui, &mut ui_state.selected_material_index, matl.entries.len(), |i| {
                                matl.entries
                                    .get(i)
                                    .map(|m| m.material_label.clone())
                                    .unwrap_or_default()
                            });

                        if ui_state.matl_editor_advanced_mode && ui.button("Delete").clicked() {
                            // TODO: Potential panic?
                            matl.entries.remove(ui_state.selected_material_index);
                        }
                    });
                    // Advanced mode has more detailed information that most users won't want to edit.
                    ui.checkbox(&mut ui_state.matl_editor_advanced_mode, "Advanced Settings");

                    if let Some(entry) = matl.entries.get_mut(ui_state.selected_material_index) {
                        // TODO: Avoid collect here?
                        // Keep track of modl entries since materials may be renamed.
                        let mut modl_entries: Vec<_> = modl
                            .map(|m| {
                                m.entries
                                    .iter_mut()
                                    .filter(|e| e.material_label == entry.material_label)
                                    .collect()
                            })
                            .unwrap_or_default();

                        matl_entry_editor(
                            ui,
                            entry,
                            &mut modl_entries,
                            folder_thumbnails,
                            default_thumbnails,
                            ui_state.matl_editor_advanced_mode,
                            shader_database,
                        );
                    }
                });
        });

    open
}

fn shader_label(ui: &mut egui::Ui, shader_label: &str, is_valid: bool) {
    if is_valid {
        ui.label(shader_label);
    } else {
        ui.horizontal(|ui| {
            // TODO: Add a black/red checkerboard for clarity.
            error_icon(ui);
            ui.label(egui::RichText::new(shader_label).color(egui::Color32::RED));
        })
        .response
        .on_hover_text(format!("{} is not a valid shader label.", shader_label));
    }
}

fn matl_entry_editor(
    ui: &mut egui::Ui,
    entry: &mut ssbh_data::matl_data::MatlEntryData,
    modl_entries: &mut [&mut ModlEntryData],
    texture_thumbnails: &[(String, egui::TextureId)],
    default_thumbnails: &[(String, egui::TextureId)],
    advanced_mode: bool,
    shader_database: &ShaderDatabase,
) {
    let program = shader_database.get(entry.shader_label.get(..24).unwrap_or(""));

    if advanced_mode {
        ui.label("Shader Label");
        ui.indent("shader indent", |ui| {
            shader_label(ui, &entry.shader_label, program.is_some());
            egui::Grid::new("shader_grid").show(ui, |ui| {
                // TODO: Should this be part of the basic mode.
                // TODO: Should advanced mode just be a textbox?
                ui.label("Render Pass");
                let shader = entry.shader_label.get(..24).unwrap_or("").to_string();
                egui::ComboBox::from_id_source("render pass")
                    .selected_text(entry.shader_label.get(25..).unwrap_or(""))
                    .show_ui(ui, |ui| {
                        for pass in [
                            shader.clone() + "_opaque",
                            shader.clone() + "_far",
                            shader.clone() + "_sort",
                            shader.clone() + "_near",
                        ] {
                            ui.selectable_value(
                                &mut entry.shader_label,
                                pass.to_string(),
                                pass.get(25..).unwrap_or(""),
                            );
                        }
                    });
                ui.end_row();

                // TODO: Get info from shader database.
                if let Some(program) = program {
                    ui.label("Alpha Testing");
                    ui.label(program.discard.to_string());
                    ui.end_row();

                    ui.label("Vertex Attributes");
                    ui.add(egui::Label::new(program.vertex_attributes.join(",")).wrap(true));
                    ui.end_row();
                }
            });
        });
    } else {
        ui.horizontal(|ui| {
            ui.label("Shader Label");
            shader_label(ui, &entry.shader_label, program.is_some());
        });
    }

    ui.horizontal(|ui| {
        ui.label("Material Label");
        // TODO: Get this to work with lost_focus for efficiency.
        if ui.text_edit_singleline(&mut entry.material_label).changed() {
            // Rename any effected modl entries if the material label changes.
            for modl_entry in modl_entries {
                modl_entry.material_label = entry.material_label.clone();
            }
        }
    });

    // TODO: Option to delete unneeded parameters.
    if let Some(program) = program {
        let missing_parameters = missing_parameters(entry, program);
        if !missing_parameters.is_empty() && ui.button("Add Missing Parameters").clicked() {
            add_parameters(entry, &missing_parameters);
        }

        let unused_parameters = unused_parameters(entry, program);
        if !unused_parameters.is_empty() && ui.button("Remove Unused Parameters").clicked() {
            remove_parameters(entry, &unused_parameters);
        }
    }

    for param in entry.booleans.iter_mut() {
        ui.checkbox(&mut param.data, param.param_id.to_string());
    }

    for param in entry.floats.iter_mut() {
        ui.horizontal(|ui| {
            // TODO: Store this size somewhere to ensure labels align?
            ui.add_sized([80.0, 20.0], egui::Label::new(param.param_id.to_string()));
            ui.add(egui::Slider::new(&mut param.data, 0.0..=1.0));
        });
    }

    if advanced_mode {
        for param in entry.vectors.iter_mut() {
            // TODO: Make a custom expander that expands to sliders?
            // TODO: Set custom labels and ranges.
            // TODO: Add parameter descriptions.
            ui.label(param.param_id.to_string());

            ui.indent("indent", |ui| {
                egui::Grid::new(param.param_id.to_string()).show(ui, |ui| {
                    ui.label("X");
                    ui.add(egui::Slider::new(&mut param.data.x, 0.0..=1.0).clamp_to_range(false));
                    ui.end_row();

                    ui.label("Y");
                    ui.add(egui::Slider::new(&mut param.data.y, 0.0..=1.0).clamp_to_range(false));
                    ui.end_row();

                    ui.label("Z");
                    ui.add(egui::Slider::new(&mut param.data.z, 0.0..=1.0).clamp_to_range(false));
                    ui.end_row();

                    ui.label("W");
                    ui.add(egui::Slider::new(&mut param.data.w, 0.0..=1.0).clamp_to_range(false));
                    ui.end_row();
                });
            });
        }
    } else {
        egui::Grid::new("vectors").show(ui, |ui| {
            for param in entry.vectors.iter_mut() {
                // TODO: Store this size somewhere to ensure labels align?
                ui.add_sized([80.0, 20.0], egui::Label::new(param.param_id.to_string()));

                let mut color = [param.data.x, param.data.y, param.data.z];
                if ui.color_edit_button_rgb(&mut color).changed() {
                    param.data.x = color[0];
                    param.data.y = color[1];
                    param.data.z = color[2];
                }
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(egui::DragValue::new(&mut param.data.x));
                    ui.label("Y");
                    ui.add(egui::DragValue::new(&mut param.data.y));
                    ui.label("Z");
                    ui.add(egui::DragValue::new(&mut param.data.z));
                    ui.label("W");
                    ui.add(egui::DragValue::new(&mut param.data.w));
                });

                ui.end_row();
            }
        });
    }

    // The defaults for samplers are usually fine, so don't show samplers by default.
    if advanced_mode {
        for param in &mut entry.samplers {
            ui.label(param.param_id.to_string());
            ui.indent("indent", |ui| {
                egui::Grid::new(param.param_id.to_string()).show(ui, |ui| {
                    enum_combo_box(
                        ui,
                        "Wrap S",
                        format!("wraps{:?}", param.param_id),
                        &mut param.data.wraps,
                    );
                    ui.end_row();

                    enum_combo_box(
                        ui,
                        "Wrap T",
                        format!("wrapt{:?}", param.param_id),
                        &mut param.data.wrapt,
                    );
                    ui.end_row();

                    enum_combo_box(
                        ui,
                        "Wrap R",
                        format!("wrapr{:?}", param.param_id),
                        &mut param.data.wrapr,
                    );
                    ui.end_row();

                    enum_combo_box(
                        ui,
                        "Min Filter",
                        format!("minfilter{:?}", param.param_id),
                        &mut param.data.min_filter,
                    );
                    ui.end_row();

                    enum_combo_box(
                        ui,
                        "Mag Filter",
                        format!("magfilter{:?}", param.param_id),
                        &mut param.data.mag_filter,
                    );
                    ui.end_row();
                });
            });
        }
    }

    for param in &mut entry.textures {
        // TODO: Should this check be case sensitive?
        // TODO: Create a texture for an invalid thumbnail or missing texture?
        // TODO: Should this functionality be part of ssbh_wgpu?
        ui.horizontal(|ui| {
            ui.add_sized([80.0, 20.0], egui::Label::new(param.param_id.to_string()));

            // TODO: How to handle #replace_cubemap?
            // Texture parameters don't include the file extension since it's implied.
            if let Some(thumbnail) = texture_thumbnails
                .iter()
                .chain(default_thumbnails.iter())
                .find(|t| Path::new(&t.0).with_extension("") == Path::new(&param.data))
                .map(|t| t.1)
            {
                ui.image(thumbnail, egui::Vec2::new(24.0, 24.0));
            }

            if advanced_mode {
                // Let users enter names manually if texture files aren't present.
                ui.text_edit_singleline(&mut param.data);
            } else {
                // Texture files should be present in the folder, which allows for image previews.
                egui::ComboBox::from_id_source(param.param_id.to_string())
                    .selected_text(&param.data)
                    .show_ui(ui, |ui| {
                        // TODO: Is it safe to assume the thumbnails have all the available textures?
                        for (name, thumbnail) in
                            texture_thumbnails.iter().chain(default_thumbnails.iter())
                        {
                            // Material parameters don't include the .nutexb extension.
                            let text = Path::new(name)
                                .with_extension("")
                                .to_string_lossy()
                                .to_string();

                            ui.horizontal(|ui| {
                                ui.image(*thumbnail, egui::Vec2::new(24.0, 24.0));
                                ui.selectable_value(&mut param.data, text.to_string(), text);
                            });
                        }
                    });
            }
        });
    }

    // TODO: Reflecting changes to these values in the viewport requires recreating pipelines.
    // Most users will want to leave the rasterizer state at its default values.
    if advanced_mode {
        for param in &mut entry.rasterizer_states {
            // TODO: These param IDs might not be unique?
            ui.label(param.param_id.to_string());

            ui.indent("todo1", |ui| {
                egui::Grid::new(param.param_id.to_string()).show(ui, |ui| {
                    enum_combo_box(
                        ui,
                        "Polygon Fill",
                        format!("fill{:?}", param.param_id.to_string()),
                        &mut param.data.fill_mode,
                    );
                    ui.end_row();
                    enum_combo_box(
                        ui,
                        "Cull Mode",
                        format!("cull{:?}", param.param_id.to_string()),
                        &mut param.data.cull_mode,
                    );
                    ui.end_row();
                });
            });
        }
    }

    for param in &mut entry.blend_states {
        ui.label(param.param_id.to_string());
        ui.indent("blend state", |ui| {
            egui::Grid::new(param.param_id.to_string()).show(ui, |ui| {
                enum_combo_box(
                    ui,
                    "Source Color",
                    format!("srccolor{:?}", param.param_id.to_string()),
                    &mut param.data.source_color,
                );
                ui.end_row();

                enum_combo_box(
                    ui,
                    "Destination Color",
                    format!("dstcolor{:?}", param.param_id.to_string()),
                    &mut param.data.destination_color,
                );
                ui.end_row();

                ui.checkbox(
                    &mut param.data.alpha_sample_to_coverage,
                    "Alpha Sample to Coverage",
                );
                ui.end_row();
                // TODO: Basic blend state can just expose a selection for "additive", "alpha", or "opaque".
                // TODO: Research in game examples for these presets (premultiplied alpha?)
            });
        });
    }
}

// TODO: Animation Viewer
// Users want to know what values are being effected, see the values, and toggle tracks on/off.
// The display could be done using egui's plotting capabilities using Blender as a reference.
