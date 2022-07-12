use egui::{
    remap_clamp, CursorIcon, Key, Rect, Response, Sense, Stroke, TextEdit, TextStyle, Ui, Vec2,
    Widget, WidgetText,
};

/// A widget similar to an [egui::DragValue]
/// that fills up as the value approaches the max.
pub struct DragSlider<'a> {
    value: &'a mut f32,
    width: f32,
}

// TODO: Use the builder pattern?
impl<'a> DragSlider<'a> {
    pub fn new(value: &'a mut f32, width: f32) -> Self {
        DragSlider { value, width }
    }
}

// Based on a DragValue.
// https://github.com/emilk/egui/blob/master/egui/src/widgets/drag_value.rs
impl<'a> Widget for DragSlider<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let DragSlider { value, width } = self;

        // TODO: Use the button height?
        let desired_size = Vec2::new(width, 20.0);

        // TODO: Support more than one drag slider?
        let kb_edit_id = ui.make_persistent_id("drag_slider_edit");
        let edit_text_id = kb_edit_id.with("text");

        // Switch from a slider to a text edit on click.
        // Return to using a slider if the text edit loses focus.
        let response = if ui.memory().has_focus(kb_edit_id) {
            // TODO: Do we need a separate ID for this?
            let mut value_text = ui
                .memory()
                .data
                .get_temp::<String>(edit_text_id)
                .unwrap_or(value.to_string());
            let response = ui.add(
                TextEdit::singleline(&mut value_text)
                    .id(kb_edit_id)
                    .desired_width(desired_size.x),
            );

            if ui.input().key_pressed(Key::Enter) {
                // TODO: Also update value on lost focus.
                if let Ok(new_value) = value_text.parse() {
                    *value = new_value;
                }

                ui.memory().surrender_focus(edit_text_id);
                ui.memory().data.remove::<String>(edit_text_id);
            } else {
                ui.memory()
                    .data
                    .insert_temp::<String>(edit_text_id, value_text);
            }
            response
        } else {
            let (outer_rect, response) =
                ui.allocate_exact_size(desired_size, Sense::click_and_drag());

            if response.clicked() {
                // TODO: Select all in the text edit on initial focus?
                ui.memory().request_focus(kb_edit_id);
                // Remove stale values if present.
                ui.memory().data.remove::<String>(edit_text_id);
            } else if response.dragged() {
                ui.output().cursor_icon = CursorIcon::ResizeHorizontal;

                // Fill the bar up to the cursor location similar to a slider.
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    // TODO: Custom value range?
                    let delta_value = remap_clamp(
                        pointer_pos.x,
                        outer_rect.left()..=outer_rect.right(),
                        0.0..=1.0,
                    );
                    *value = delta_value.clamp(0.0, 1.0);
                }
            }

            if ui.is_rect_visible(outer_rect) {
                let visuals = ui.style().interact(&response);

                ui.painter().rect(
                    outer_rect.expand(visuals.expansion),
                    visuals.rounding,
                    visuals.bg_fill,
                    visuals.bg_stroke,
                );

                let fill_amount = value.clamp(0.0, 1.0);
                let inner_rect = Rect::from_min_size(
                    outer_rect.min,
                    Vec2::new(
                        outer_rect.width() * fill_amount.clamp(0.0, 1.0),
                        outer_rect.height(),
                    ),
                );

                ui.painter().rect(
                    inner_rect,
                    visuals.rounding,
                    ui.visuals().selection.bg_fill,
                    Stroke::none(),
                );

                // Limit the displayed digits while still preserving precision.
                let text = WidgetText::from(format!("{:.3}", value)).into_galley(
                    ui,
                    None,
                    desired_size.x,
                    TextStyle::Button,
                );

                // Center the text in the slider rect.
                // TODO: Will this always be the right layout.
                let text_pos = ui
                    .layout()
                    .align_size_within_rect(text.size(), outer_rect)
                    .min;

                text.paint_with_visuals(ui.painter(), text_pos, visuals);
            }
            response
        };

        response
    }
}
