use crate::{
    editors::{
        hlpb::hlpb_editor, matl::matl_editor, mesh::mesh_editor, modl::modl_editor,
        skel::skel_editor,
    },
    horizontal_separator_empty, load_model, load_models_recursive,
    widgets::*,
};
use egui::{collapsing_header::CollapsingState, CollapsingHeader, ScrollArea};
use lazy_static::lazy_static;
use log::Log;
use rfd::FileDialog;
use ssbh_data::{matl_data::MatlEntryData, prelude::*};
use ssbh_wgpu::{
    DebugMode, ModelFolder, PipelineData, RenderModel, RenderSettings, ShaderDatabase,
};
use std::{path::Path, str::FromStr, sync::Mutex};
use strum::VariantNames;

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
        metadata.level() <= log::Level::Warn
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

    pub red_checkerboard: egui::TextureId,
    pub yellow_checkerboard: egui::TextureId,

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
    pub mesh_editor_advanced_mode: bool,
    pub log_window_open: bool,
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

pub struct AnimationIndex {
    pub folder_index: usize,
    pub anim_index: usize,
}

impl AnimationIndex {
    pub fn get_animation<'a>(
        index: Option<&AnimationIndex>,
        models: &'a [ModelFolder],
    ) -> Option<&'a (String, AnimData)> {
        index.and_then(|index| {
            models
                .get(index.folder_index)
                .and_then(|m| m.anims.get(index.anim_index))
        })
    }
}

pub struct AnimationState {
    pub current_frame: f32,
    pub is_playing: bool,
    pub animation_frame_was_changed: bool,
    pub selected_slot: usize,
    pub animations: Vec<Option<AnimationIndex>>,
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
    pub fn open_folder(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            // TODO: Sort alphabetically?
            // TODO: Allow for opening folders with no mesh/modl?
            self.models = load_models_recursive(folder);
            self.should_refresh_meshes = true;
        }
    }

    pub fn add_folder_to_workspace(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            // Load the folder manually to avoid skipping folders with just animations.
            // TODO: Is there an easier way to allow loading animation folders?
            let new_model = load_model(&folder);
            self.models.push(new_model);
            // TODO: Only update the model that was added?
            self.should_refresh_meshes = true;
        }
    }

    pub fn reload_workspace(&mut self) {
        // This also reloads animations since animations are stored as indices.
        for model in &mut self.models {
            *model = ModelFolder::load_folder(&model.folder_name);
        }
        self.should_refresh_meshes = true;
    }

    pub fn clear_workspace(&mut self) {
        self.models = Vec::new();
        self.render_models = Vec::new();
        self.thumbnails = Vec::new();
        self.animation_state.animations = Vec::new();
        // TODO: Reset selected indices?
        // TODO: Is there an easy way to write this?
    }
}

const ICON_SIZE: f32 = 18.0;

impl SsbhApp {
    pub fn update(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    let button =
                        |ui: &mut egui::Ui, text| ui.add(egui::Button::new(text).wrap(false));

                    if button(ui, "Open Folder...    (Ctrl+O)").clicked() {
                        ui.close_menu();
                        self.open_folder();
                    }

                    if button(ui, "Add Folder to Workspace...").clicked() {
                        ui.close_menu();
                        self.add_folder_to_workspace();
                    }

                    if button(ui, "Reload Workspace    (Ctrl+R)").clicked() {
                        ui.close_menu();
                        self.reload_workspace();
                    }

                    if button(ui, "Clear Workspace").clicked() {
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

                egui::menu::menu_button(ui, "Help", |ui| {
                    if ui.button("GitHub Repository").clicked() {
                        ui.close_menu();
                        let link = "https://github.com/ScanMountGoat/ssbh_editor";
                        if let Err(open_err) = open::that(link) {
                            log::error!("Failed to open link ({link}). {open_err}");
                        }
                    }

                    if ui.button("Report Issues").clicked() {
                        ui.close_menu();
                        let link = "https://github.com/ScanMountGoat/ssbh_editor/issues";
                        if let Err(open_err) = open::that(link) {
                            log::error!("Failed to open link ({link}). {open_err}");
                        }
                    }

                    if ui.button("Changelog").clicked() {
                        ui.close_menu();
                        let link =
                            "https://github.com/ScanMountGoat/ssbh_editor/blob/main/CHANGELOG.md";
                        if let Err(open_err) = open::that(link) {
                            log::error!("Failed to open link ({link}). {open_err}");
                        }
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

        log_window(ctx, &mut self.ui_state.log_window_open);

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
                        let release_link = "https://github.com/ScanMountGoat/ssbh_editor/releases";
                        if ui.hyperlink(release_link).clicked() {
                            if ui.hyperlink(release_link).clicked() {
                                if let Err(open_err) = open::that(release_link) {
                                    log::error!(
                                    "Failed to open link ({release_link}) to releases {open_err}"
                                );
                                }
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
                        if !mesh_editor(
                            ctx,
                            &display_name(&model.folder_name, name),
                            mesh,
                            &mut self.ui_state.mesh_editor_advanced_mode,
                        ) {
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
                            model
                                .meshes
                                .iter()
                                .find(|(f, _)| f == "model.numshb")
                                .map(|(_, m)| m),
                            &self.thumbnails[folder_index],
                            &self.default_thumbnails,
                            &self.render_state.shader_database,
                            &self.material_presets,
                            self.red_checkerboard,
                            self.yellow_checkerboard,
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
                            CollapsingHeader::new(folder_display_name(model))
                                .id_source(format!("folder.{}", folder_index))
                                .default_open(true)
                                .show(ui, |ui| {
                                    // TODO: Show a visual indication if a file has warnings/errors.
                                    // This will encourage users to click and investigate the errors.

                                    // Avoid a confusing missing file error for animation folders.
                                    let just_anim = model.meshes.is_empty()
                                        && model.modls.is_empty()
                                        && model.skels.is_empty()
                                        && model.matls.is_empty()
                                        && !model.anims.is_empty();

                                    let required_file =
                                        |name| if just_anim { None } else { Some(name) };

                                    // Clicking a file opens the corresponding editor.
                                    // Set selected index so the editor remains open for the file.
                                    // TODO: Should the index be cleared when reloading models?
                                    list_files(
                                        ui,
                                        &model.meshes,
                                        folder_index,
                                        &mut self.ui_state.selected_folder_index,
                                        &mut self.ui_state.selected_mesh_index,
                                        required_file("model.numshb"),
                                    );

                                    list_files(
                                        ui,
                                        &model.skels,
                                        folder_index,
                                        &mut self.ui_state.selected_folder_index,
                                        &mut self.ui_state.selected_skel_index,
                                        required_file("model.nusktb"),
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
                                        required_file("model.numatb"),
                                    );

                                    list_files(
                                        ui,
                                        &model.modls,
                                        folder_index,
                                        &mut self.ui_state.selected_folder_index,
                                        &mut self.ui_state.selected_modl_index,
                                        required_file("model.numdlb"),
                                    );

                                    for (i, (name, _)) in model.anims.iter().enumerate() {
                                        ui.horizontal(|ui| {
                                            empty_icon(ui);
                                            if ui.button(name).clicked() {
                                                let animation = AnimationIndex {
                                                    folder_index,
                                                    anim_index: i,
                                                };

                                                // Create the first slot if it doesn't exist to save mouse clicks.
                                                if self.animation_state.animations.is_empty() {
                                                    self.animation_state
                                                        .animations
                                                        .push(Some(animation));
                                                } else if let Some(slot) = self
                                                    .animation_state
                                                    .animations
                                                    .get_mut(self.animation_state.selected_slot)
                                                {
                                                    *slot = Some(animation);
                                                }

                                                // Preview the new animation as soon as it is clicked.
                                                self.animation_state.animation_frame_was_changed =
                                                    true;
                                            }
                                        });
                                    }

                                    // TODO: Display larger versions when clicking?
                                    // TODO: How to manage the thumbnails?
                                    // TODO: Cube map thumbnails.
                                    // TODO: Register wgpu textures as is without converting to RGBA?
                                    // TODO: Add a warning for nutexbs with unused padding (requires tegra_swizzle update).
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

        egui::TopBottomPanel::bottom("bottom panel").show(ctx, |ui| {
            ui.with_layout(
                egui::Layout::left_to_right().with_cross_align(egui::Align::Center),
                |ui| {
                    self.animation_bar(ui);

                    // The next layout needs to be min since it's nested inside a centered layout.
                    ui.with_layout(
                        egui::Layout::right_to_left().with_cross_align(egui::Align::Min),
                        |ui| {
                            ui.horizontal(|ui| {
                                if let Some((level, message)) =
                                    LOGGER.messages.lock().unwrap().last()
                                {
                                    if ui
                                        .add_sized([60.0, 30.0], egui::Button::new("Logs"))
                                        .clicked()
                                    {
                                        self.ui_state.log_window_open = true;
                                    }

                                    // Clicking the message also opens the log window.
                                    let abbreviated_message =
                                        message.get(..40).unwrap_or_default().to_string() + "...";
                                    if ui
                                        .add(
                                            egui::Label::new(abbreviated_message)
                                                .sense(egui::Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        self.ui_state.log_window_open = true;
                                    }

                                    match level {
                                        log::Level::Error => error_icon(ui),
                                        log::Level::Warn => warning_icon(ui),
                                        log::Level::Info => (),
                                        log::Level::Debug => (),
                                        log::Level::Trace => (),
                                    }
                                }
                            });
                        },
                    );
                },
            );
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
                        PanelTab::MeshList => mesh_list(ctx, self, ui),
                        PanelTab::AnimList => anim_list(ctx, self, ui),
                    });
            });
    }

    fn animation_bar(&mut self, ui: &mut egui::Ui) {
        let final_frame_index = self.max_final_frame_index();

        // TODO: How to fill available space?
        // TODO: Get the space that would normally be taken up by the central panel?
        ui.spacing_mut().slider_width = (ui.available_width() - 520.0).max(0.0);
        if ui
            .add(
                egui::Slider::new(
                    &mut self.animation_state.current_frame,
                    0.0..=final_frame_index,
                )
                .step_by(1.0)
                .show_value(false),
            )
            .changed()
        {
            // Manually trigger an update in case the playback is paused.
            self.animation_state.animation_frame_was_changed = true;
        }

        // Use a separate widget from the slider value to force the size.
        // This reduces the chances of the widget resizing during animations.
        if ui
            .add_sized(
                [60.0, 20.0],
                egui::DragValue::new(&mut self.animation_state.current_frame),
            )
            .changed()
        {
            // Manually trigger an update in case the playback is paused.
            self.animation_state.animation_frame_was_changed = true;
        }

        // Nest these conditions to avoid displaying both "Pause" and "Play" at once.
        let size = [60.0, 30.0];

        // TODO: Checkbox for looping?
        // TODO: Playback speed?
        if self.animation_state.is_playing {
            if ui.add_sized(size, egui::Button::new("Pause")).clicked() {
                self.animation_state.is_playing = false;
            }
        } else if ui.add_sized(size, egui::Button::new("Play")).clicked() {
            self.animation_state.is_playing = true;
        }
    }

    pub fn max_final_frame_index(&mut self) -> f32 {
        // Find the minimum number of frames to cover all animations.
        let mut final_frame_index = 0.0;
        for anim_index in &self.animation_state.animations {
            if let Some((_, anim)) =
                AnimationIndex::get_animation(anim_index.as_ref(), &self.models)
            {
                if anim.final_frame_index > final_frame_index {
                    final_frame_index = anim.final_frame_index;
                }
            }
        }
        final_frame_index
    }
}

fn folder_display_name(model: &ModelFolder) -> String {
    Path::new(&model.folder_name)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
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

pub fn empty_icon(ui: &mut egui::Ui) {
    ui.allocate_space(egui::Vec2::new(ICON_SIZE, ICON_SIZE));
}

pub fn missing_icon(ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("⚠").size(ICON_SIZE));
}

pub fn warning_icon(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("⚠")
            .strong()
            .color(egui::Color32::from_rgb(255, 210, 0))
            .size(ICON_SIZE),
    );
}

pub fn error_icon(ui: &mut egui::Ui) {
    ui.label(
        egui::RichText::new("⚠")
            .strong()
            .color(egui::Color32::from_rgb(200, 40, 40))
            .size(ICON_SIZE),
    );
}

fn mesh_list(ctx: &egui::Context, app: &mut SsbhApp, ui: &mut egui::Ui) {
    // TODO: Display folders that only have animations differently?
    for (i, folder) in app.models.iter().enumerate() {
        let name = format!("meshlist.{}", i);

        let id = ui.make_persistent_id(&name);
        CollapsingState::load_with_default_open(ctx, id, true)
            .show_header(ui, |ui| {
                if let Some(render_model) = app.render_models.get_mut(i) {
                    ui.add(EyeCheckBox::new(
                        &mut render_model.is_visible,
                        &folder_display_name(folder),
                    ));
                }
            })
            .body(|ui| {
                // TODO: How to ensure the render models stay in sync with the model folder?
                if let Some(render_model) = app.render_models.get_mut(i) {
                    for mesh in &mut render_model.meshes {
                        ui.add(EyeCheckBox::new(&mut mesh.is_visible, &mesh.name));
                    }
                }
            });
    }
}

fn anim_list(ctx: &egui::Context, app: &mut SsbhApp, ui: &mut egui::Ui) {
    // TODO: Will these IDs be unique?
    if ui.button("Add Slot").clicked() {
        app.animation_state.animations.push(None);
    }

    let mut slots_to_remove = Vec::new();
    for (i, anim_index) in app.animation_state.animations.iter().enumerate().rev() {
        // TODO: Unique IDs?
        let id = ui.make_persistent_id(i);
        CollapsingState::load_with_default_open(ctx, id, false)
            .show_header(ui, |ui| {
                let name = AnimationIndex::get_animation(anim_index.as_ref(), &app.models)
                    .map(|(name, _)| name.to_string())
                    .unwrap_or_default();

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
            })
            .body(|ui| {
                if let Some((_, anim)) =
                    AnimationIndex::get_animation(anim_index.as_ref(), &app.models)
                {
                    for group in &anim.groups {
                        CollapsingHeader::new(group.group_type.to_string())
                            .default_open(true)
                            .show(ui, |ui| {
                                for node in &group.nodes {
                                    match node.tracks.as_slice() {
                                        [_] => {
                                            // Don't use a collapsing header if there is only one track.
                                            // This simplifies the layout for most boolean and transform tracks.
                                            // TODO: How to toggle visibility for rendering?
                                            ui.label(&node.name);
                                        }
                                        _ => {
                                            CollapsingHeader::new(&node.name)
                                                .default_open(true)
                                                .show(ui, |ui| {
                                                    for track in &node.tracks {
                                                        // TODO: How to toggle visibility for rendering?
                                                        ui.label(&track.name);
                                                    }
                                                });
                                        }
                                    }
                                }
                            });
                    }
                }
            });
    }

    // TODO: Force only one slot to be removed?
    for slot in slots_to_remove {
        app.animation_state.animations.remove(slot);
    }
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
                    egui::Grid::new("debug_shading_grid").show(ui, |ui| {
                        // TODO: Add descriptions.
                        ui.label("Debug Mode");
                        egui::ComboBox::from_id_source("Debug Mode")
                            .width(200.0)
                            .selected_text(debug_mode_label(settings.debug_mode))
                            .show_ui(ui, |ui| {
                                for name in DebugMode::VARIANTS {
                                    let variant = DebugMode::from_str(name).unwrap();
                                    ui.selectable_value(
                                        &mut settings.debug_mode,
                                        variant,
                                        debug_mode_label(variant),
                                    );
                                }
                            });

                        ui.end_row();

                        if settings.debug_mode == ssbh_wgpu::DebugMode::Shaded {
                            enum_combo_box(
                                ui,
                                "Transition Material",
                                "Transition Material",
                                &mut settings.transition_material,
                            );
                            ui.end_row();

                            ui.label("Transition Factor");
                            ui.add(egui::Slider::new(
                                &mut settings.transition_factor,
                                0.0..=1.0,
                            ));
                            ui.end_row();
                        }
                    });
                    if settings.debug_mode != DebugMode::Shaded {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut settings.render_rgba[0], "R");
                            ui.checkbox(&mut settings.render_rgba[1], "G");
                            ui.checkbox(&mut settings.render_rgba[2], "B");
                            ui.checkbox(&mut settings.render_rgba[3], "A");
                        });
                    }
                    horizontal_separator_empty(ui);

                    ui.heading("Render Passes");
                    ui.checkbox(&mut settings.render_diffuse, "Enable Diffuse");
                    ui.checkbox(&mut settings.render_specular, "Enable Specular");
                    ui.checkbox(&mut settings.render_emission, "Enable Emission");
                    ui.checkbox(&mut settings.render_rim_lighting, "Enable Rim Lighting");
                    ui.checkbox(&mut settings.render_bloom, "Enable Bloom");
                    horizontal_separator_empty(ui);

                    ui.heading("Lighting");
                    ui.checkbox(&mut settings.render_shadows, "Enable Shadows");
                    horizontal_separator_empty(ui);
                });
        });
}

fn debug_mode_label(mode: DebugMode) -> String {
    let description = debug_description(mode);
    if !description.is_empty() {
        format!("{} ({})", mode, description)
    } else {
        mode.to_string()
    }
}

fn debug_description(mode: DebugMode) -> &'static str {
    // TODO: Should these be identical to the material descriptions?
    match mode {
        DebugMode::Texture0 => "Col Layer 1",
        DebugMode::Texture1 => "Col Layer 2",
        DebugMode::Texture2 => "Irradiance Cube",
        DebugMode::Texture3 => "Ambient Occlusion",
        DebugMode::Texture4 => "Nor",
        DebugMode::Texture5 => "Emissive Layer 1",
        DebugMode::Texture6 => "Prm",
        DebugMode::Texture7 => "Specular Cube",
        DebugMode::Texture8 => "Diffuse Cube",
        DebugMode::Texture9 => "Baked Lighting",
        DebugMode::Texture10 => "Diffuse Layer 1",
        DebugMode::Texture11 => "Diffuse Layer 2",
        DebugMode::Texture12 => "Diffuse Layer 3",
        DebugMode::Texture14 => "Emissive Layer 2",
        _ => "",
    }
}

fn log_window(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("Application Log")
        .open(open)
        .resizable(true)
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (level, message) in LOGGER.messages.lock().unwrap().iter() {
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
        });
}

// TODO: Animation Viewer
// Users want to know what values are being effected, see the values, and toggle tracks on/off.
// The display could be done using egui's plotting capabilities using Blender as a reference.
