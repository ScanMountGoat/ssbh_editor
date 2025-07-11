use crate::{
    app::{AnimEditorState, AnimEditorTab},
    path::folder_editor_title,
    save_file, save_file_as, EditorResponse,
};
use egui::{
    special_emojis::GITHUB, CentralPanel, CollapsingHeader, DragValue, RichText, ScrollArea,
    SidePanel,
};
use egui_extras::{Column, TableBuilder};
use egui_plot::{Legend, Line, Plot, PlotPoint};

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
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save").clicked() {
                        saved |= save_file(anim, folder_name, file_name);
                    }

                    if ui.button("Save As...").clicked() {
                        saved |= save_file_as(anim, folder_name, file_name, "Anim", "nuanmb");
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button(format!("{GITHUB} Anim Editor Wiki")).clicked() {
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
        message: None,
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
            .legend(Legend::default().follow_insertion_order(true));

        let mut shapes = Vec::new();

        if let Some(track) = selected_track(&mut anim.groups, state) {
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
                    shapes.push(Line::new("translation.x", translation_x));
                    shapes.push(Line::new("translation.y", translation_y));
                    shapes.push(Line::new("translation.z", translation_z));

                    shapes.push(Line::new("rotation.x", rotation_x));
                    shapes.push(Line::new("rotation.y", rotation_y));
                    shapes.push(Line::new("rotation.z", rotation_z));
                    shapes.push(Line::new("rotation.w", rotation_w));

                    shapes.push(Line::new("scale.x", scale_x));
                    shapes.push(Line::new("scale.y", scale_y));
                    shapes.push(Line::new("scale.z", scale_z));
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
                    shapes.push(Line::new("scale_u", scale_us));
                    shapes.push(Line::new("scale_v", scale_vs));

                    shapes.push(Line::new("rotation", rotations));

                    shapes.push(Line::new("translate_u", translate_us));
                    shapes.push(Line::new("translate_v", translate_vs));
                }
                TrackValues::Float(values) => {
                    let mut points = Vec::new();
                    for (i, v) in values.iter().enumerate() {
                        points.push([i as f64, *v as f64]);
                    }
                    shapes.push(Line::new("value", points));
                }
                TrackValues::PatternIndex(values) => {
                    let mut points = Vec::new();
                    for (i, v) in values.iter().enumerate() {
                        points.push([i as f64, *v as f64]);
                    }
                    shapes.push(Line::new("value", points));
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
                    shapes.push(Line::new("value", points));
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
                    shapes.push(Line::new("x", xs));
                    shapes.push(Line::new("y", ys));
                    shapes.push(Line::new("z", zs));
                    shapes.push(Line::new("w", ws));
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

fn selected_track<'a>(
    groups: &'a mut [GroupData],
    state: &AnimEditorState,
) -> Option<&'a mut TrackData> {
    groups
        .get_mut(state.selected_group_index?)?
        .nodes
        .get_mut(state.selected_node_index?)?
        .tracks
        .get_mut(state.selected_track_index?)
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

    let mut changed = false;

    CentralPanel::default().show_inside(ui, |ui| {
        if let Some(track) = selected_track(&mut anim.groups, state) {
            changed |= track_value_grid(ui, track);
        }
    });

    changed
}

fn track_value_grid(ui: &mut egui::Ui, track: &mut TrackData) -> bool {
    let mut changed = false;

    let count = track.values.len();

    let heading = |ui: &mut egui::Ui, label: &str| {
        ui.strong(label);
    };

    let column_count = match track.values {
        TrackValues::Transform(_) => 10,
        TrackValues::UvTransform(_) => 5,
        TrackValues::Float(_) => 1,
        TrackValues::PatternIndex(_) => 1,
        TrackValues::Boolean(_) => 1,
        TrackValues::Vector4(_) => 4,
    };

    TableBuilder::new(ui)
        .striped(true)
        .cell_layout(egui::Layout::centered_and_justified(
            egui::Direction::LeftToRight,
        ))
        .column(Column::auto())
        .columns(
            Column::remainder()
                .clip(true)
                .at_least(60.0)
                .resizable(true),
            column_count,
        )
        .header(20.0, |mut header| match &track.values {
            TrackValues::Transform(_) => {
                header.col(|ui| heading(ui, "frame"));
                header.col(|ui| heading(ui, "scale.x"));
                header.col(|ui| heading(ui, "scale.y"));
                header.col(|ui| heading(ui, "scale.z"));
                header.col(|ui| heading(ui, "rotation.x"));
                header.col(|ui| heading(ui, "rotation.y"));
                header.col(|ui| heading(ui, "rotation.z"));
                header.col(|ui| heading(ui, "rotation.w"));
                header.col(|ui| heading(ui, "translation.x"));
                header.col(|ui| heading(ui, "translation.y"));
                header.col(|ui| heading(ui, "translation.z"));
            }
            TrackValues::UvTransform(_) => {
                header.col(|ui| heading(ui, "frame"));
                header.col(|ui| heading(ui, "scale_u"));
                header.col(|ui| heading(ui, "scale_v"));
                header.col(|ui| heading(ui, "rotation"));
                header.col(|ui| heading(ui, "translate_u"));
                header.col(|ui| heading(ui, "translate_v"));
            }
            TrackValues::Float(_) => {
                header.col(|ui| heading(ui, "frame"));
                header.col(|ui| heading(ui, "value"));
            }
            TrackValues::PatternIndex(_) => {
                header.col(|ui| heading(ui, "frame"));
                header.col(|ui| heading(ui, "value"));
            }
            TrackValues::Boolean(_) => {
                header.col(|ui| heading(ui, "frame"));
                header.col(|ui| heading(ui, "value"));
            }
            TrackValues::Vector4(_) => {
                header.col(|ui| heading(ui, "frame"));
                header.col(|ui| heading(ui, "x"));
                header.col(|ui| heading(ui, "y"));
                header.col(|ui| heading(ui, "z"));
                header.col(|ui| heading(ui, "w"));
            }
        })
        .body(|body| match &mut track.values {
            TrackValues::Transform(values) => {
                body.rows(20.0, count, |mut row| {
                    let mut edit_value = |ui: &mut egui::Ui, f| {
                        changed |= ui.add(DragValue::new(f).speed(0.1)).changed();
                    };

                    let i = row.index();
                    let v = &mut values[i];

                    row.col(|ui| {
                        ui.label(i.to_string());
                    });

                    row.col(|ui| edit_value(ui, &mut v.scale.x));
                    row.col(|ui| edit_value(ui, &mut v.scale.y));
                    row.col(|ui| edit_value(ui, &mut v.scale.z));

                    row.col(|ui| edit_value(ui, &mut v.rotation.x));
                    row.col(|ui| edit_value(ui, &mut v.rotation.y));
                    row.col(|ui| edit_value(ui, &mut v.rotation.z));
                    row.col(|ui| edit_value(ui, &mut v.rotation.w));

                    row.col(|ui| edit_value(ui, &mut v.translation.x));
                    row.col(|ui| edit_value(ui, &mut v.translation.y));
                    row.col(|ui| edit_value(ui, &mut v.translation.z));
                });
            }
            TrackValues::UvTransform(values) => {
                body.rows(20.0, count, |mut row| {
                    let mut edit_value = |ui: &mut egui::Ui, f| {
                        changed |= ui.add(DragValue::new(f).speed(0.1)).changed();
                    };

                    let i = row.index();
                    let v = &mut values[i];

                    row.col(|ui| {
                        ui.label(i.to_string());
                    });

                    row.col(|ui| edit_value(ui, &mut v.scale_u));
                    row.col(|ui| edit_value(ui, &mut v.scale_v));

                    row.col(|ui| edit_value(ui, &mut v.rotation));

                    row.col(|ui| edit_value(ui, &mut v.translate_u));
                    row.col(|ui| edit_value(ui, &mut v.translate_v));
                });
            }
            TrackValues::Float(values) => {
                body.rows(20.0, count, |mut row| {
                    let mut edit_value = |ui: &mut egui::Ui, f| {
                        changed |= ui.add(DragValue::new(f).speed(0.1)).changed();
                    };

                    let i = row.index();
                    let v = &mut values[i];

                    row.col(|ui| {
                        ui.label(i.to_string());
                    });

                    row.col(|ui| edit_value(ui, v));
                });
            }
            TrackValues::PatternIndex(values) => {
                body.rows(20.0, count, |mut row| {
                    let mut edit_value = |ui: &mut egui::Ui, f| {
                        changed |= ui.add(DragValue::new(f).speed(0.1)).changed();
                    };

                    let i = row.index();
                    let v = &mut values[i];

                    row.col(|ui| {
                        ui.label(i.to_string());
                    });

                    row.col(|ui| edit_value(ui, v));
                });
            }
            TrackValues::Boolean(values) => {
                body.rows(20.0, count, |mut row| {
                    let i = row.index();
                    let v = &mut values[i];

                    row.col(|ui| {
                        ui.label(i.to_string());
                    });

                    row.col(|ui| {
                        changed |= ui.checkbox(v, "").changed();
                    });
                });
            }
            TrackValues::Vector4(values) => {
                body.rows(20.0, count, |mut row| {
                    let mut edit_value = |ui: &mut egui::Ui, f| {
                        changed |= ui.add(DragValue::new(f).speed(0.1)).changed();
                    };

                    let i = row.index();
                    let v = &mut values[i];

                    row.col(|ui| {
                        ui.label(i.to_string());
                    });

                    row.col(|ui| edit_value(ui, &mut v.x));
                    row.col(|ui| edit_value(ui, &mut v.y));
                    row.col(|ui| edit_value(ui, &mut v.z));
                    row.col(|ui| edit_value(ui, &mut v.w));
                });
            }
        });

    changed
}
