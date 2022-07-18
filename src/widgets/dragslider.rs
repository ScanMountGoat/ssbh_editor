use egui::{
    text::CCursor,
    text_edit::{CCursorRange, TextEditState},
    CursorIcon, Id, Key, Rect, Response, Sense, Stroke, TextEdit, TextStyle, Ui, Vec2, Widget,
    WidgetText,
};

/// A combined slider and text edit that fills up like an [egui::ProgressBar].
pub struct DragSlider<'a> {
    id: Id,
    value: &'a mut f32,
    width: f32,
    slider_min: f32,
    slider_max: f32,
}

impl<'a> DragSlider<'a> {
    pub fn new(id_source: impl std::hash::Hash, value: &'a mut f32) -> Self {
        DragSlider {
            id: Id::new(id_source),
            value,
            width: 100.0,
            slider_min: 0.0,
            slider_max: 1.0,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn slider_range(mut self, min: f32, max: f32) -> Self {
        self.slider_min = min;
        self.slider_max = max;
        self
    }
}

// Based on a DragValue.
// https://github.com/emilk/egui/blob/master/egui/src/widgets/drag_value.rs
impl<'a> Widget for DragSlider<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        // TODO: Use the default button height?
        let desired_size = Vec2::new(self.width, 20.0);

        let kb_edit_id = self.id;
        let edit_text_id = kb_edit_id.with("text");

        // Switch from a slider to a text edit on click.
        // Return to using a slider if the text edit loses focus.
        let response = if ui.memory().has_focus(kb_edit_id) {
            // TODO: Customize the precision.
            // TODO: Does this preserve precision?
            let mut value_text = ui
                .memory()
                .data
                .get_temp::<String>(edit_text_id)
                .unwrap_or_else(|| format!("{:.3}", self.value));

            let response = ui.add(
                TextEdit::singleline(&mut value_text)
                    .id(kb_edit_id)
                    .desired_width(desired_size.x),
            );

            if ui.input().key_pressed(Key::Enter) {
                // TODO: Also update value on lost focus.
                if let Ok(new_value) = value_text.parse() {
                    *self.value = new_value;
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
            // Limit the displayed digits while still preserving precision.
            let text = WidgetText::from(format!("{:.3}", self.value)).into_galley(
                ui,
                None,
                desired_size.x,
                TextStyle::Button,
            );

            // Expand the widget as needed to fit the text.
            let text_size = text.size() + 2.0 * ui.spacing().button_padding;
            let (outer_rect, response) =
                ui.allocate_at_least(desired_size.max(text_size), Sense::click_and_drag());

            if response.clicked() {
                ui.memory().request_focus(kb_edit_id);

                // Remove stale values if present.
                ui.memory().data.remove::<String>(edit_text_id);

                // Select all when next showing the text edit.
                select_all_text(ui, kb_edit_id);
            } else if response.dragged() {
                ui.output().cursor_icon = CursorIcon::ResizeHorizontal;
                // Don't update the value if the cursor didn't move.
                // This prevents accidental value changes while clicking.
                let delta = response.drag_delta();
                if delta.length_sq() > 0.0 {
                    let delta_value =
                        delta.x * (self.slider_max - self.slider_min) /
                        (outer_rect.right() - outer_rect.left());
                    *self.value += delta_value;
                    *self.value = self.value.clamp(self.slider_min, self.slider_max);
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

                let fill_amount = self.value.clamp(0.0, 1.0);
                let inner_rect = Rect::from_min_size(
                    outer_rect.min,
                    Vec2::new(outer_rect.width() * fill_amount, outer_rect.height()),
                );

                ui.painter().rect(
                    inner_rect,
                    visuals.rounding,
                    ui.visuals().selection.bg_fill,
                    Stroke::none(),
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

fn select_all_text(ui: &mut Ui, kb_edit_id: Id) {
    // This mimics selecting text with the mouse or keyboard.
    // Choose a large index to select all text.
    let mut state = TextEditState::default();
    state.set_ccursor_range(Some(CCursorRange {
        primary: CCursor::new(1000),
        secondary: CCursor::new(0),
    }));
    TextEdit::store_state(ui.ctx(), kb_edit_id, state);
}
