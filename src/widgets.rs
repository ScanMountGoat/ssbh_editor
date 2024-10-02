use egui::{
    epaint, pos2, vec2, Image, NumExt, Response, Sense, TextStyle, TextureOptions, Ui, Widget,
    WidgetInfo, WidgetText, WidgetType,
};
use ssbh_data::skel_data::SkelData;

mod dragslider;
pub use dragslider::DragSlider;

pub struct EyeCheckBox<'a> {
    checked: &'a mut bool,
    text: WidgetText,
}

impl<'a> EyeCheckBox<'a> {
    pub fn new(checked: &'a mut bool, text: impl Into<WidgetText>) -> Self {
        EyeCheckBox {
            checked,
            text: text.into(),
        }
    }
}

impl<'a> Widget for EyeCheckBox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let EyeCheckBox { checked, text } = self;

        ui.spacing_mut().icon_width = 18.0;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = ui.spacing().icon_spacing;
        let button_padding = spacing.button_padding;
        let total_extra = button_padding + vec2(icon_width + icon_spacing, 0.0) + button_padding;

        let wrap_width = ui.available_width() - total_extra.x;
        let text = text.into_galley(ui, None, wrap_width, TextStyle::Button);
        let mut desired_size = total_extra + text.size();
        desired_size = desired_size.at_least(spacing.interact_size);
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            *checked = !*checked;
            response.mark_changed();
        }
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::Checkbox, true, *checked, text.text())
        });

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let text_pos = pos2(
                rect.min.x + button_padding.x + icon_width + icon_spacing,
                rect.center().y - 0.5 * text.size().y,
            );
            let (_small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
            ui.painter().add(epaint::RectShape::filled(
                big_icon_rect.expand(visuals.expansion),
                visuals.rounding,
                visuals.bg_fill,
            ));

            if *checked {
                eye_open_icon(ui, big_icon_rect, visuals);
            } else {
                // TODO: closed eye icon.
            }

            ui.painter().galley(text_pos, text, visuals.text_color());
        }

        response
    }
}

fn eye_open_icon(ui: &Ui, rect: egui::Rect, visuals: &egui::style::WidgetVisuals) {
    // Render at twice the desired size to handle high DPI displays.
    let image = egui::include_image!("icons/eye_visibility_open.svg");
    match image
        .load(
            ui.ctx(),
            TextureOptions::default(),
            egui::SizeHint::Size(rect.width() as u32 * 2, rect.height() as u32 * 2),
        )
        .unwrap()
    {
        egui::load::TexturePoll::Pending { .. } => (),
        egui::load::TexturePoll::Ready { texture } => Image::new(texture)
            .tint(visuals.text_color())
            .paint_at(ui, rect),
    }
}

pub fn enum_combo_box<V>(ui: &mut egui::Ui, id_source: impl std::hash::Hash, value: &mut V) -> bool
where
    V: PartialEq + strum::IntoEnumIterator + ToString,
{
    // TODO: Return response and union instead?
    let mut changed = false;
    egui::ComboBox::from_id_salt(id_source)
        .width(ui.available_width())
        .selected_text(value.to_string())
        .show_ui(ui, |ui| {
            for v in V::iter() {
                let text = v.to_string();
                changed |= ui.selectable_value(value, v, text).changed();
            }
        });

    changed
}

pub fn bone_combo_box(
    ui: &mut egui::Ui,
    bone_name: &mut String,
    id: impl std::hash::Hash,
    skel: Option<&SkelData>,
    extra_names: &[&str],
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(bone_name.clone())
        .show_ui(ui, |ui| {
            for name in extra_names {
                changed |= ui
                    .selectable_value(bone_name, name.to_string(), *name)
                    .changed();
            }

            if let Some(skel) = skel {
                for bone in &skel.bones {
                    changed |= ui
                        .selectable_value(bone_name, bone.name.clone(), &bone.name)
                        .changed();
                }
            } else {
                changed |= ui.text_edit_singleline(bone_name).changed();
            }
        });
    changed
}
