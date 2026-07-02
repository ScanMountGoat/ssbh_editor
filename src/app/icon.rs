use egui::{ImageSource, Label, Response, RichText, Ui};

use crate::{
    TEXT_COLOR_DARK, TEXT_COLOR_LIGHT,
    app::{ERROR_COLOR, ICON_SIZE, ICON_TEXT_SIZE, WARNING_COLOR},
};

// All the icons are designed to render properly at 16x16 pixels.
pub fn draggable_icon(ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(
        ui,
        egui::include_image!("../icons/carbon_draggable.svg"),
        dark_mode,
    )
}

pub fn mesh_icon(ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ui, egui::include_image!("../icons/mesh.svg"), dark_mode)
}

pub fn matl_icon(ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ui, egui::include_image!("../icons/matl.svg"), dark_mode)
}

pub fn adj_icon(ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ui, egui::include_image!("../icons/adj.svg"), dark_mode)
}

pub fn anim_icon(ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ui, egui::include_image!("../icons/anim.svg"), dark_mode)
}

pub fn skel_icon(ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ui, egui::include_image!("../icons/skel.svg"), dark_mode)
}

pub fn hlpb_icon(ui: &mut Ui, dark_mode: bool) -> Response {
    file_icon(ui, egui::include_image!("../icons/hlpb.svg"), dark_mode)
}

fn file_icon(ui: &mut Ui, image: ImageSource, dark_mode: bool) -> Response {
    let tint = if dark_mode {
        TEXT_COLOR_DARK
    } else {
        TEXT_COLOR_LIGHT
    };

    ui.add(
        egui::Image::new(image)
            .tint(tint)
            .fit_to_exact_size(egui::vec2(16.0, 16.0)),
    )
}

pub fn empty_icon(ui: &mut Ui) {
    ui.allocate_space(egui::Vec2::new(ICON_SIZE, ICON_SIZE));
}

pub fn missing_icon(ui: &mut Ui) -> Response {
    ui.add_sized(
        [ICON_SIZE, ICON_SIZE],
        Label::new(RichText::new("⚠").size(ICON_TEXT_SIZE)),
    )
}

pub fn warning_icon(ui: &mut Ui) -> Response {
    let text = RichText::new("⚠").strong().color(WARNING_COLOR);
    let label = Label::new(text.size(ICON_TEXT_SIZE));
    ui.add_sized([ICON_SIZE, ICON_SIZE], label)
}

pub fn error_icon(ui: &mut Ui) -> Response {
    let text = RichText::new("⚠").strong().color(ERROR_COLOR);
    let label = Label::new(text.size(ICON_TEXT_SIZE));
    ui.add_sized([ICON_SIZE, ICON_SIZE], label)
}
