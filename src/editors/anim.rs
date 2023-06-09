use crate::{
    app::{AnimEditorState, AnimEditorTab},
    path::folder_editor_title,
    EditorResponse, save_file, save_file_as,
};
use egui::{
    plot::{Legend, Line, Plot, PlotPoint},
    special_emojis::GITHUB,
    CentralPanel, CollapsingHeader, Grid, RichText, ScrollArea, SidePanel,
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
    folder_name: &Path,
    file_name: &str,
    anim: &mut AnimData,
    state: &mut AnimEditorState,
) -> EditorResponse {
    let mut open = true;
    let mut changed = false;
    let mut saved = false;

    let title = folder_editor_title(folder_name, file_name);
    egui::Window::new(format!("Anim Editor ({title})"))
        .default_width(800.0)
        .default_height(600.0)
        .open(&mut open)
        .resizable(true)
        .show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        ui.close_menu();
                        saved |= save_file(anim, folder_name, file_name);
                    }

                    if ui.button("Save As...").clicked() {
                        ui.close_menu();
                        saved |= save_file_as(anim, folder_name, file_name, "Anim", "nuanmb");
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} Anim Editor Wiki")).clicked() {
                        ui.close_menu();

                        let link = "https://github.com/ScanMountGoat/ssbh_editor/wiki/Anim-Editor";
                        if let Err(e) = open::that(link) {
                            log::error!("Failed to open {link}: {e}");
                        }
                    }
                });
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.editor_tab,
                    AnimEditorTab::Hierarchy,
                    RichText::new("Hierarchy").heading(),
                );
                ui.selectable_value(
                    &mut state.editor_tab,
                    AnimEditorTab::Graph,
                    RichText::new("Graph").heading(),
                );
                ui.selectable_value(
                    &mut state.editor_tab,
                    AnimEditorTab::List,
                    RichText::new("List").heading(),
                );
            });

            changed |= match state.editor_tab {
                AnimEditorTab::Hierarchy => hierarchy_view(ui, anim, state),
                AnimEditorTab::Graph => graph_view(ui, anim, state),
                AnimEditorTab::List => list_view(ui, anim, state),
            };
        });

    EditorResponse {
        open,
        changed,
        saved,
    }
}

fn hierarchy_view(ui: &mut egui::Ui, anim: &mut AnimData, _state: &mut AnimEditorState) -> bool {
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
    select_track_panel(ui, anim, state);

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
                    let mut scale_us = Vec::new();
                    let mut scale_vs = Vec::new();
                    let mut rotations = Vec::new();
                    let mut translate_us = Vec::new();
                    let mut translate_vs = Vec::new();

                    for (i, v) in values.iter().enumerate() {
                        scale_us.push([i as f64, v.scale_u as f64]);
                        scale_vs.push([i as f64, v.scale_v as f64]);

                        rotations.push([i as f64, v.rotation as f64]);

                        translate_us.push([i as f64, v.translate_u as f64]);
                        translate_vs.push([i as f64, v.translate_v as f64]);
                    }
                    shapes.push(Line::new(scale_us).name("scale_u"));
                    shapes.push(Line::new(scale_vs).name("scale_v"));

                    shapes.push(Line::new(rotations).name("rotation"));

                    shapes.push(Line::new(translate_us).name("translate_u"));
                    shapes.push(Line::new(translate_vs).name("translate_v"));
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
                    shapes.push(Line::new(points).name("value"));
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

    // The graph view is readonly.
    false
}

fn select_track_panel(ui: &mut egui::Ui, anim: &mut AnimData, state: &mut AnimEditorState) {
    SidePanel::left("anim_left_panel")
        .default_width(300.0)
        .show_inside(ui, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for (g, group) in anim.groups.iter().enumerate() {
                        CollapsingHeader::new(group.group_type.to_string())
                            .default_open(false)
                            .show(ui, |ui| {
                                for (n, node) in &mut group.nodes.iter().enumerate() {
                                    match &node.tracks[..] {
                                        [t] => {
                                            // Single tracks just use the group type as the name.
                                            // Make the node selectable instead.
                                            let mut selected = state.selected_group_index
                                                == Some(g)
                                                && state.selected_node_index == Some(n)
                                                && state.selected_track_index == Some(0);
                                            ui.selectable_value(
                                                &mut selected,
                                                true,
                                                format!(
                                                    "{} ({} frames)",
                                                    node.name,
                                                    t.values.len()
                                                ),
                                            );
                                            if selected {
                                                state.selected_group_index = Some(g);
                                                state.selected_node_index = Some(n);
                                                state.selected_track_index = Some(0);
                                            }
                                        }
                                        tracks => {
                                            CollapsingHeader::new(&node.name)
                                                .default_open(true)
                                                .show(ui, |ui| {
                                                    for (t, track) in tracks.iter().enumerate() {
                                                        let mut selected = state
                                                            .selected_group_index
                                                            == Some(g)
                                                            && state.selected_node_index == Some(n)
                                                            && state.selected_track_index
                                                                == Some(t);
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
                                                });
                                        }
                                    }
                                }
                            });
                    }
                });
        });
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
            .checkbox(&mut track.compensate_scale, "Compensate Scale")
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

        // TODO: Double check if this is accurately named.
        changed |= ui
            .checkbox(
                &mut track.transform_flags.override_compensate_scale,
                "Override Compensate Scale",
            )
            .changed();
    });

    changed
}

fn list_view(ui: &mut egui::Ui, anim: &mut AnimData, state: &mut AnimEditorState) -> bool {
    select_track_panel(ui, anim, state);

    CentralPanel::default().show_inside(ui, |ui| {
        if let Some(track) = selected_track(&anim.groups, state) {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    track_value_grid(ui, track);
                });
        }
    });

    // The list view is readonly.
    false
}

fn track_value_grid(ui: &mut egui::Ui, track: &TrackData) {
    Grid::new("anim_grid")
        .striped(true)
        .show(ui, |ui| match &track.values {
            TrackValues::Transform(values) => {
                ui.heading("scale.x");
                ui.heading("scale.y");
                ui.heading("scale.z");
                ui.heading("rotation.x");
                ui.heading("rotation.y");
                ui.heading("rotation.z");
                ui.heading("rotation.w");
                ui.heading("translation.x");
                ui.heading("translation.y");
                ui.heading("translation.z");
                ui.end_row();

                for v in values {
                    ui.label(v.scale.x.to_string());
                    ui.label(v.scale.y.to_string());
                    ui.label(v.scale.z.to_string());

                    ui.label(v.rotation.x.to_string());
                    ui.label(v.rotation.y.to_string());
                    ui.label(v.rotation.z.to_string());
                    ui.label(v.rotation.w.to_string());

                    ui.label(v.translation.x.to_string());
                    ui.label(v.translation.y.to_string());
                    ui.label(v.translation.z.to_string());

                    ui.end_row();
                }
            }
            TrackValues::UvTransform(values) => {
                ui.heading("scale_u");
                ui.heading("scale_v");
                ui.heading("rotation");
                ui.heading("translate_u");
                ui.heading("translate_v");
                ui.end_row();

                for v in values {
                    ui.label(v.scale_u.to_string());
                    ui.label(v.scale_v.to_string());

                    ui.label(v.rotation.to_string());

                    ui.label(v.translate_u.to_string());
                    ui.label(v.translate_v.to_string());

                    ui.end_row();
                }
            }
            TrackValues::Float(values) => {
                ui.heading("value");
                ui.end_row();

                for v in values {
                    ui.label(v.to_string());
                    ui.end_row();
                }
            }
            TrackValues::PatternIndex(values) => {
                ui.heading("value");
                ui.end_row();

                for v in values {
                    ui.label(v.to_string());
                    ui.end_row();
                }
            }
            TrackValues::Boolean(values) => {
                ui.heading("value");
                ui.end_row();

                for v in values {
                    ui.label(v.to_string());
                    ui.end_row();
                }
            }
            TrackValues::Vector4(values) => {
                ui.heading("x");
                ui.heading("y");
                ui.heading("z");
                ui.heading("w");

                ui.end_row();

                for v in values {
                    ui.label(v.x.to_string());
                    ui.label(v.y.to_string());
                    ui.label(v.z.to_string());
                    ui.label(v.w.to_string());
                    ui.end_row();
                }
            }
        });
}
