use std::str::FromStr;

use egui::{
    epaint, pos2, vec2, NumExt, Response, RichText, Sense, TextStyle, Ui, Widget, WidgetInfo,
    WidgetText, WidgetType,
};

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
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
        response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, *checked, text.text()));

        if ui.is_rect_visible(rect) {
            // let visuals = ui.style().interact_selectable(&response, *checked); // too colorful
            let visuals = ui.style().interact(&response);
            let text_pos = pos2(
                rect.min.x + button_padding.x + icon_width + icon_spacing,
                rect.center().y - 0.5 * text.size().y,
            );
            let (_small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
            ui.painter().add(epaint::RectShape {
                rect: big_icon_rect.expand(visuals.expansion),
                rounding: visuals.rounding,
                fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
            });

            if *checked {
                // TODO: Use a custom shape?
                // TODO: Make this easier to see when hidden (add a closed eye icon like blender?)
                let eye_text = WidgetText::RichText(RichText::new("👁").size(20.0)).into_galley(
                    ui,
                    None,
                    wrap_width,
                    TextStyle::Button,
                );
                // TODO: How to center this?
                let eye_text_pos = pos2(big_icon_rect.min.x - 1.0, big_icon_rect.min.y + 2.0);
                eye_text.paint_with_visuals(ui.painter(), eye_text_pos, visuals);
            }

            text.paint_with_visuals(ui.painter(), text_pos, visuals);
        }

        response
    }
}

pub fn enum_combo_box<V>(
    ui: &mut egui::Ui,
    label: &str,
    id_source: impl std::hash::Hash,
    value: &mut V,
) where
    V: PartialEq + strum::VariantNames + ToString + FromStr,
    <V as FromStr>::Err: std::fmt::Debug,
{
    if label != "" {
        ui.label(label);
    }

    egui::ComboBox::from_id_source(id_source)
        .width(200.0)
        .selected_text(value.to_string())
        .show_ui(ui, |ui| {
            // TODO: Does the performance cost here matter?
            for v in V::VARIANTS {
                ui.selectable_value(value, V::from_str(v).unwrap(), v.to_string());
            }
        });
}
