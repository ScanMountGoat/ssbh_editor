use crate::app::{AnimEditorState, AnimEditorTab};
use egui::{
    plot::{Legend, Line, Plot, PlotPoint},
    CentralPanel, CollapsingHeader, RichText, ScrollArea, SidePanel,
};
use log::error;
use rfd::FileDialog;
use ssbh_data::{
    anim_data::{GroupData, TrackData, TrackValues},
    prelude::*,
};
use std::path::Path;

pub fn anim_editor(
    ctx: &egui::Context,
    title: &str,
    folder_name: &str,
    file_name: &str,
    anim: &mut AnimData,
    state: &mut AnimEditorState,
) -> (bool, bool) {
    let mut open = true;
    let mut changed = false;

    egui::Window::new(format!("Anim Editor ({title})"))
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();

                        let file = Path::new(folder_name).join(file_name);
                        if let Err(e) = anim.write_to_file(&file) {
                            error!("Failed to save {:?}: {}", file, e);
                        }
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();

                        if let Some(file) = FileDialog::new()
                            .add_filter("Anim", &["nuanmb"])
                            .save_file()
                        {
                            if let Err(e) = anim.write_to_file(&file) {
                                error!("Failed to save {:?}: {}", file, e);
                            }
                        }
                    }
                });

                egui::menu::menu_button(ui, "Help", |ui| {
                    if ui.button("Anim Editor Wiki").clicked() {
                        ui.close_menu();

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Anim-Editor";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            // TODO: Modes for edit, graph edit, and dope sheet?
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.editor_tab,
                    AnimEditorTab::Editor,
                    RichText::new("Editor").heading(),
                );
                ui.selectable_value(
                    &mut state.editor_tab,
                    AnimEditorTab::Graph,
                    RichText::new("Graph").heading(),
                );
            });

            changed |= match state.editor_tab {
                AnimEditorTab::Editor => editor_view(ui, anim, state),
                AnimEditorTab::Graph => graph_view(ui, anim, state),
            };
        });

    (open, changed)
}

fn editor_view(ui: &mut egui::Ui, anim: &mut AnimData, _state: &mut AnimEditorState) -> bool {
    let mut changed = false;
    ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            // TODO: Make names editable?
            for group in &mut anim.groups {
                CollapsingHeader::new(group.group_type.to_string())
                    .default_open(false)
                    .show(ui, |ui| {
                        for node in &mut group.nodes {
                            CollapsingHeader::new(&node.name)
                                .default_open(true)
                                .show(ui, |ui| {
                                    for track in &mut node.tracks {
                                        changed |= edit_track(ui, track);
                                    }
                                });
                        }
                    });
            }
        });
    changed
}

fn graph_view(ui: &mut egui::Ui, anim: &mut AnimData, state: &mut AnimEditorState) -> bool {
    let mut _changed = false;

    SidePanel::left("anim_left_panel").show_inside(ui, |ui| {
        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // TODO: Make names editable in edit mode?
                for (g, group) in &mut anim.groups.iter().enumerate() {
                    CollapsingHeader::new(group.group_type.to_string())
                        .default_open(false)
                        .show(ui, |ui| {
                            for (n, node) in &mut group.nodes.iter().enumerate() {
                                match &node.tracks[..] {
                                    [t] => {
                                        // Single tracks just use the group type as the name.
                                        // Make the node selectable instead.
                                        let mut selected = state.selected_group_index == Some(g)
                                            && state.selected_node_index == Some(n)
                                            && state.selected_track_index == Some(0);
                                        ui.selectable_value(
                                            &mut selected,
                                            true,
                                            format!("{} ({} frames)", node.name, t.values.len()),
                                        );
                                        if selected {
                                            state.selected_group_index = Some(g);
                                            state.selected_node_index = Some(n);
                                            state.selected_track_index = Some(0);
                                        }
                                    }
                                    tracks => {
                                        CollapsingHeader::new(&node.name).default_open(true).show(
                                            ui,
                                            |ui| {
                                                for (t, track) in tracks.iter().enumerate() {
                                                    // TODO: Should multiple tracks be viewable at once?
                                                    // TODO: Easier to just have a selected group, node, track?
                                                    let mut selected = state.selected_group_index
                                                        == Some(g)
                                                        && state.selected_node_index == Some(n)
                                                        && state.selected_track_index == Some(t);
                                                    ui.selectable_value(
                                                        &mut selected,
                                                        true,
                                                        format!(
                                                            "{} ({} frames)",
                                                            track.name,
                                                            track.values.len()
                                                        ),
                                                    );
                                                    if selected {
                                                        state.selected_group_index = Some(g);
                                                        state.selected_node_index = Some(n);
                                                        state.selected_track_index = Some(t);
                                                    }
                                                }
                                            },
                                        );
                                    }
                                }
                            }
                        });
                }
            });
    });

    CentralPanel::default().show_inside(ui, |ui| {
        let label_fmt = |name: &str, value: &PlotPoint| {
            if name.is_empty() {
                // Don't show values when not hovering near a line.
                String::new()
            } else {
                format!("{name}\nframe = {}\nvalue = {}", value.x, value.y)
            }
        };

        // Add a legend for labels and visibility toggles
        let plot = Plot::new("anim_plot")
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .label_formatter(label_fmt)
            .legend(Legend::default());

        let mut shapes = Vec::new();

        if let Some(track) = selected_track(&anim.groups, state) {
            match &track.values {
                TrackValues::Transform(values) => {
                    let mut translation_x = Vec::new();
                    let mut translation_y = Vec::new();
                    let mut translation_z = Vec::new();

                    let mut rotation_x = Vec::new();
                    let mut rotation_y = Vec::new();
                    let mut rotation_z = Vec::new();
                    let mut rotation_w = Vec::new();

                    let mut scale_x = Vec::new();
                    let mut scale_y = Vec::new();
                    let mut scale_z = Vec::new();

                    for (i, t) in values.iter().enumerate() {
                        translation_x.push([i as f64, t.translation.x as f64]);
                        translation_y.push([i as f64, t.translation.y as f64]);
                        translation_z.push([i as f64, t.translation.z as f64]);

                        rotation_x.push([i as f64, t.rotation.x as f64]);
                        rotation_y.push([i as f64, t.rotation.y as f64]);
                        rotation_z.push([i as f64, t.rotation.z as f64]);
                        rotation_w.push([i as f64, t.rotation.w as f64]);

                        scale_x.push([i as f64, t.scale.x as f64]);
                        scale_y.push([i as f64, t.scale.y as f64]);
                        scale_z.push([i as f64, t.scale.z as f64]);
                    }
                    shapes.push(Line::new(translation_x).name("translation.x"));
                    shapes.push(Line::new(translation_y).name("translation.y"));
                    shapes.push(Line::new(translation_z).name("translation.z"));

                    shapes.push(Line::new(rotation_x).name("rotation.x"));
                    shapes.push(Line::new(rotation_y).name("rotation.y"));
                    shapes.push(Line::new(rotation_z).name("rotation.z"));
                    shapes.push(Line::new(rotation_w).name("rotation.w"));

                    shapes.push(Line::new(scale_x).name("scale.x"));
                    shapes.push(Line::new(scale_y).name("scale.y"));
                    shapes.push(Line::new(scale_z).name("scale.z"));
                }
                TrackValues::UvTransform(values) => {
                    let mut translate_us = Vec::new();
                    for (i, v) in values.iter().enumerate() {
                        translate_us.push([i as f64, v.translate_u as f64]);
                    }
                    shapes.push(Line::new(translate_us).name("translate_u"));
                }
                TrackValues::Float(values) => {
                    let mut points = Vec::new();
                    for (i, v) in values.iter().enumerate() {
                        points.push([i as f64, *v as f64]);
                    }
                    shapes.push(Line::new(points).name("value"));
                }
                TrackValues::PatternIndex(values) => {
                    let mut points = Vec::new();
                    for (i, v) in values.iter().enumerate() {
                        points.push([i as f64, *v as f64]);
                    }
                    shapes.push(Line::new(points).name("pattern index"));
                }
                TrackValues::Boolean(values) => {
                    let mut points = Vec::new();
                    for (i, v) in values.iter().enumerate() {
                        points.push([i as f64, if *v { 1.0 } else { 0.0 }]);
                        // Each value lasts until the next frame.
                        if i < values.len() - 1 {
                            points.push([(i + 1) as f64, if *v { 1.0 } else { 0.0 }]);
                        }
                    }
                    shapes.push(Line::new(points).name("value"));
                }
                TrackValues::Vector4(values) => {
                    let mut xs = Vec::new();
                    let mut ys = Vec::new();
                    let mut zs = Vec::new();
                    let mut ws = Vec::new();

                    for (i, v) in values.iter().enumerate() {
                        xs.push([i as f64, v.x as f64]);
                        ys.push([i as f64, v.y as f64]);
                        zs.push([i as f64, v.z as f64]);
                        ws.push([i as f64, v.w as f64]);
                    }
                    // TODO: use the track name here as well?
                    shapes.push(Line::new(xs).name("x"));
                    shapes.push(Line::new(ys).name("y"));
                    shapes.push(Line::new(zs).name("z"));
                    shapes.push(Line::new(ws).name("w"));
                }
            }
        }

        plot.show(ui, |plot_ui| {
            for shape in shapes {
                plot_ui.line(shape);
            }
        });
    });

    _changed
}

fn selected_track<'a>(groups: &'a [GroupData], state: &AnimEditorState) -> Option<&'a TrackData> {
    groups
        .get(state.selected_group_index?)?
        .nodes
        .get(state.selected_node_index?)?
        .tracks
        .get(state.selected_track_index?)
}

fn edit_track(ui: &mut egui::Ui, track: &mut ssbh_data::anim_data::TrackData) -> bool {
    let mut changed = false;

    ui.label(&track.name);
    ui.indent("indent", |ui| {
        changed |= ui
            .checkbox(
                &mut track.scale_options.compensate_scale,
                "Compensate Scale",
            )
            .changed();

        changed |= ui
            .checkbox(&mut track.scale_options.inherit_scale, "Inherit Scale")
            .changed();

        changed |= ui
            .checkbox(
                &mut track.transform_flags.override_translation,
                "Override Translation",
            )
            .changed();

        changed |= ui
            .checkbox(
                &mut track.transform_flags.override_rotation,
                "Override Rotation",
            )
            .changed();

        changed |= ui
            .checkbox(&mut track.transform_flags.override_scale, "Override Scale")
            .changed();
    });

    changed
}
